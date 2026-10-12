//! Gera as variantes de uma imagem enviada: AVIF e WebP com perdas, em
//! larguras fixas, sem ampliar além do original.
//!
//! A função é síncrona e pesada (o AVIF leva segundos por imagem). Quem chama
//! decide a fila e a thread.

use std::io::Cursor;

use image::codecs::avif::AvifEncoder;
use image::imageops::FilterType;
use image::{
    DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageReader, ImageResult,
};
use sha2::{Digest, Sha256};

use crate::tipos::Formato;
use crate::validacao::{Gravidade, Problema, normalizar_slug, validar_upload};

/// Larguras geradas, quando o original comporta.
pub const LARGURAS: [u32; 4] = [480, 768, 1200, 1920];

const QUALIDADE_WEBP: f32 = 78.0;
/// De 1 a 100: mais alto fica mais perto do original e pesa mais.
const QUALIDADE_AVIF: u8 = 63;
/// De 1 (lento, arquivo menor) a 10 (rápido).
const VELOCIDADE_AVIF: u8 = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarianteGerada {
    pub formato: Formato,
    pub largura: u32,
    /// `<nome>-<largura>.<formato>`.
    pub arquivo: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagemProcessada {
    /// Nome amigável mais um trecho do hash do original, para cache longo.
    pub nome: String,
    /// Dimensões depois de aplicada a orientação.
    pub largura: u32,
    pub altura: u32,
    pub variantes: Vec<VarianteGerada>,
}

pub fn processar_imagem(
    nome_original: &str,
    bytes: &[u8],
) -> Result<ImagemProcessada, Vec<Problema>> {
    let problemas = validar_upload(nome_original, bytes);
    if !problemas.is_empty() {
        return Err(problemas);
    }
    let ilegivel = || {
        vec![Problema {
            codigo: "upload.ilegivel",
            campo: nome_original.to_string(),
            gravidade: Gravidade::Bloqueia,
            mensagem:
                "Não foi possível abrir esta imagem. O arquivo pode estar corrompido: envie outro."
                    .into(),
        }]
    };

    let imagem = decodificar(bytes).map_err(|_| ilegivel())?;
    let (largura, altura) = (imagem.width(), imagem.height());
    if largura == 0 || altura == 0 {
        return Err(ilegivel());
    }

    let nome = format!("{}-{}", nome_amigavel(nome_original), trecho_do_hash(bytes));
    let mut variantes = Vec::new();
    for largura_da_variante in larguras_para(largura) {
        let altura_da_variante =
            (u64::from(altura) * u64::from(largura_da_variante) / u64::from(largura)).max(1) as u32;
        let reduzida = if largura_da_variante == largura {
            imagem.to_rgba8()
        } else {
            imagem
                .resize_exact(
                    largura_da_variante,
                    altura_da_variante,
                    FilterType::Lanczos3,
                )
                .to_rgba8()
        };

        let mut avif = Vec::new();
        AvifEncoder::new_with_speed_quality(&mut avif, VELOCIDADE_AVIF, QUALIDADE_AVIF)
            .write_image(
                reduzida.as_raw(),
                reduzida.width(),
                reduzida.height(),
                ExtendedColorType::Rgba8,
            )
            .map_err(|_| ilegivel())?;
        let webp = webp::Encoder::from_rgba(reduzida.as_raw(), reduzida.width(), reduzida.height())
            .encode(QUALIDADE_WEBP)
            .to_vec();

        for (formato, bytes) in [(Formato::Avif, avif), (Formato::Webp, webp)] {
            variantes.push(VarianteGerada {
                formato,
                largura: largura_da_variante,
                arquivo: format!("{nome}-{largura_da_variante}.{}", formato.extensao()),
                bytes,
            });
        }
    }

    Ok(ImagemProcessada {
        nome,
        largura,
        altura,
        variantes,
    })
}

/// Abre a imagem e aplica a orientação do EXIF. Os metadados não seguem
/// adiante: as variantes são codificadas só a partir dos pixels.
fn decodificar(bytes: &[u8]) -> ImageResult<DynamicImage> {
    let mut decodificador = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .into_decoder()?;
    let orientacao = decodificador.orientation()?;
    let mut imagem = DynamicImage::from_decoder(decodificador)?;
    imagem.apply_orientation(orientacao);
    Ok(imagem)
}

/// As larguras fixas que cabem no original, mais a largura dele quando é
/// menor que a maior fixa.
pub fn larguras_para(largura_original: u32) -> Vec<u32> {
    let mut larguras: Vec<u32> = LARGURAS
        .into_iter()
        .filter(|l| *l <= largura_original)
        .collect();
    if largura_original < LARGURAS[LARGURAS.len() - 1] && !larguras.contains(&largura_original) {
        larguras.push(largura_original);
    }
    larguras
}

fn nome_amigavel(nome_original: &str) -> String {
    let arquivo = nome_original.rsplit(['/', '\\']).next().unwrap_or("");
    let sem_extensao = arquivo.rsplit_once('.').map_or(arquivo, |(nome, _)| nome);
    let slug = normalizar_slug(sem_extensao);
    if slug.is_empty() {
        "imagem".into()
    } else {
        slug
    }
}

fn trecho_do_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage};

    fn jpeg(largura: u32, altura: u32) -> Vec<u8> {
        let imagem = RgbImage::from_fn(largura, altura, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, 120])
        });
        let mut bytes = Vec::new();
        DynamicImage::ImageRgb8(imagem)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Jpeg)
            .expect("JPEG de teste");
        bytes
    }

    /// Acrescenta um bloco EXIF com a orientação pedida logo depois do início do JPEG.
    fn com_orientacao(jpeg: &[u8], orientacao: u8) -> Vec<u8> {
        let mut exif = b"Exif\0\0".to_vec();
        exif.extend([0x49, 0x49, 0x2A, 0x00, 8, 0, 0, 0]); // TIFF, ordem Intel, primeiro diretório no byte 8
        exif.extend([1, 0]); // uma entrada
        exif.extend([0x12, 0x01, 3, 0, 1, 0, 0, 0, orientacao, 0, 0, 0]); // 0x0112 = orientação
        exif.extend([0, 0, 0, 0]); // sem próximo diretório
        let tamanho = (exif.len() + 2) as u16;

        let mut saida = jpeg[..2].to_vec();
        saida.extend([0xFF, 0xE1]);
        saida.extend(tamanho.to_be_bytes());
        saida.extend(exif);
        saida.extend(&jpeg[2..]);
        saida
    }

    fn dimensoes(variante: &VarianteGerada) -> (u32, u32) {
        let imagem = image::load_from_memory(&variante.bytes).expect("variante legível");
        (imagem.width(), imagem.height())
    }

    #[test]
    fn larguras_nao_ampliam_o_original() {
        assert_eq!(larguras_para(4000), [480, 768, 1200, 1920]);
        assert_eq!(larguras_para(1920), [480, 768, 1200, 1920]);
        assert_eq!(larguras_para(1000), [480, 768, 1000]);
        assert_eq!(larguras_para(768), [480, 768]);
        assert_eq!(larguras_para(300), [300]);
    }

    #[test]
    fn nome_vira_slug_com_trecho_do_hash() {
        let bytes = jpeg(64, 48);
        let imagem = processar_imagem("C:\\Fotos\\Mesa de Jantar (versão 2).JPG", &bytes)
            .expect("imagem válida");
        let (slug, hash) = imagem.nome.rsplit_once('-').expect("nome com hash");
        assert_eq!(slug, "mesa-de-jantar-versao-2");
        assert_eq!(hash.len(), 8);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        let de_novo = processar_imagem("outro-nome.jpg", &bytes).expect("imagem válida");
        assert!(
            de_novo.nome.ends_with(hash),
            "o hash é do conteúdo, não do nome"
        );
        assert!(
            processar_imagem("???.jpg", &bytes)
                .expect("imagem válida")
                .nome
                .starts_with("imagem-")
        );
    }

    #[test]
    fn imagem_pequena_gera_so_a_propria_largura_nos_dois_formatos() {
        let imagem = processar_imagem("pequena.jpg", &jpeg(300, 200)).expect("imagem válida");
        assert_eq!((imagem.largura, imagem.altura), (300, 200));
        let formatos: Vec<(Formato, u32)> = imagem
            .variantes
            .iter()
            .map(|v| (v.formato, v.largura))
            .collect();
        assert_eq!(formatos, [(Formato::Avif, 300), (Formato::Webp, 300)]);
        assert_eq!(
            imagem.variantes[1].arquivo,
            format!("{}-300.webp", imagem.nome)
        );

        // O WebP tem de ser legível e ter as dimensões pedidas.
        assert_eq!(dimensoes(&imagem.variantes[1]), (300, 200));
        // AVIF: caixa `ftyp` com a marca `avif`.
        assert_eq!(&imagem.variantes[0].bytes[4..12], b"ftypavif");
    }

    #[test]
    fn imagem_grande_e_reduzida_mantendo_a_proporcao() {
        let imagem = processar_imagem("grande.jpg", &jpeg(1000, 500)).expect("imagem válida");
        let webp: Vec<&VarianteGerada> = imagem
            .variantes
            .iter()
            .filter(|v| v.formato == Formato::Webp)
            .collect();
        assert_eq!(
            webp.iter().map(|v| dimensoes(v)).collect::<Vec<_>>(),
            [(480, 240), (768, 384), (1000, 500)]
        );
        assert_eq!(imagem.variantes.len(), 6);
    }

    #[test]
    fn orientacao_do_exif_e_aplicada() {
        // Orientação 6: a foto foi tirada com a câmera em pé.
        let bytes = com_orientacao(&jpeg(400, 200), 6);
        let imagem = processar_imagem("em-pe.jpg", &bytes).expect("imagem válida");
        assert_eq!((imagem.largura, imagem.altura), (200, 400));
        assert_eq!(dimensoes(&imagem.variantes[1]), (200, 400));
    }

    #[test]
    fn arquivo_corrompido_vira_problema() {
        let mut bytes = jpeg(64, 48);
        bytes.truncate(40);
        let problemas = processar_imagem("quebrada.jpg", &bytes).expect_err("arquivo truncado");
        assert_eq!(problemas.len(), 1);
        assert_eq!(problemas[0].codigo, "upload.ilegivel");
    }

    #[test]
    fn formato_nao_aceito_nem_chega_a_ser_aberto() {
        let problemas =
            processar_imagem("texto.jpg", b"isto nao e imagem").expect_err("não é imagem");
        assert_eq!(problemas[0].codigo, "upload.formato");
    }
}
