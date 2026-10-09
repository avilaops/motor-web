//! As travas. Rascunho salva sempre; quem barra é a publicação.
//!
//! A validação devolve a lista de problemas, com os avisos junto. Não usa
//! `Result` porque o `Ok` esconderia os avisos justamente quando a página
//! pode ser publicada.

use std::fmt;

use serde::Serialize;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;
use unicode_segmentation::UnicodeSegmentation;

use crate::tipos::{Bloco, Conteudo, Documento, Midia};
use crate::url::{incorporacao_de_video, link_valido};

pub const TITULO_RECOMENDADO: usize = 60;
pub const TITULO_MAXIMO: usize = 70;
pub const DESCRICAO_RECOMENDADA: usize = 155;
pub const DESCRICAO_MAXIMA: usize = 170;
pub const LARGURA_MINIMA_DA_CAPA: u32 = 1200;
pub const UPLOAD_MAXIMO_BYTES: usize = 15 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Gravidade {
    /// Impede publicar.
    Bloqueia,
    /// Aparece para quem edita, sem impedir.
    Avisa,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problema {
    /// Estável: é por ele que painel, API e conector reconhecem a regra.
    pub codigo: &'static str,
    /// Caminho do campo, como `corpo[3].midia.alt`.
    pub campo: String,
    pub gravidade: Gravidade,
    /// Para leigo: diz o que fazer.
    pub mensagem: String,
}

impl Problema {
    fn bloqueia(
        codigo: &'static str,
        campo: impl Into<String>,
        mensagem: impl Into<String>,
    ) -> Self {
        Self {
            codigo,
            campo: campo.into(),
            gravidade: Gravidade::Bloqueia,
            mensagem: mensagem.into(),
        }
    }

    fn avisa(codigo: &'static str, campo: impl Into<String>, mensagem: impl Into<String>) -> Self {
        Self {
            codigo,
            campo: campo.into(),
            gravidade: Gravidade::Avisa,
            mensagem: mensagem.into(),
        }
    }
}

/// O que a regra precisa saber do resto do site. Quem chama monta; o motor
/// não consulta nada.
#[derive(Debug, Clone, Copy)]
pub struct Contexto<'a> {
    /// Títulos de busca dos outros documentos do site.
    pub titulos_em_uso: &'a [String],
    pub descricoes_em_uso: &'a [String],
    /// Slug com que o documento está publicado hoje, se estiver.
    pub slug_publicado: Option<&'a str>,
    /// Slugs antigos que já têm redirecionamento.
    pub redirecionados: &'a [String],
    /// Peso medido da página, em kB.
    pub peso_kb: Option<u32>,
    pub orcamento_peso_kb: u32,
}

impl Contexto<'static> {
    /// Sem informação do resto do site: só as regras do próprio documento.
    pub fn vazio() -> Self {
        Self {
            titulos_em_uso: &[],
            descricoes_em_uso: &[],
            slug_publicado: None,
            redirecionados: &[],
            peso_kb: None,
            orcamento_peso_kb: u32::MAX,
        }
    }
}

pub fn pode_publicar(problemas: &[Problema]) -> bool {
    problemas.iter().all(|p| p.gravidade != Gravidade::Bloqueia)
}

/// Tira acento, baixa a caixa e troca o que não for letra ou número por
/// hífen, sem hífen repetido nem nas pontas.
pub fn normalizar_slug(texto: &str) -> String {
    let mut slug = String::with_capacity(texto.len());
    // Decomposição de compatibilidade: "ª" vira "a", "ﬁ" vira "fi".
    for caractere in texto.nfkd().filter(|c| !is_combining_mark(*c)) {
        if caractere.is_ascii_alphanumeric() {
            slug.push(caractere.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

/// Comprimento como a pessoa vê: "Ação" tem quatro.
pub fn comprimento(texto: &str) -> usize {
    texto.nfc().collect::<String>().graphemes(true).count()
}

/// "Atualizado em" só muda quando o que o visitante lê mudou.
pub fn deve_atualizar_data(antes: &Documento, depois: &Documento) -> bool {
    match (&antes.conteudo, &depois.conteudo) {
        (Conteudo::Pagina(a), Conteudo::Pagina(d)) => {
            a.titulo != d.titulo || a.corpo != d.corpo || a.capa != d.capa
        }
        (Conteudo::Post(a), Conteudo::Post(d)) => {
            a.titulo != d.titulo || a.resumo != d.resumo || a.corpo != d.corpo || a.capa != d.capa
        }
        (Conteudo::Produto(a), Conteudo::Produto(d)) => {
            a.nome != d.nome
                || a.descricao != d.descricao
                || a.imagens != d.imagens
                || a.variacoes != d.variacoes
        }
        _ => true,
    }
}

pub fn validar(documento: &Documento, contexto: &Contexto) -> Vec<Problema> {
    let mut problemas = Vec::new();

    validar_busca(documento, contexto, &mut problemas);
    validar_slug(documento, contexto, &mut problemas);
    if documento.titulo().trim().is_empty() {
        let campo = if matches!(documento.conteudo, Conteudo::Produto(_)) {
            "nome"
        } else {
            "titulo"
        };
        problemas.push(Problema::bloqueia(
            "titulo.vazio",
            campo,
            "Dê um título. É ele que aparece no topo da página.",
        ));
    }
    validar_corpo(documento.corpo(), &mut problemas);

    match &documento.conteudo {
        Conteudo::Pagina(pagina) => {
            if let Some(capa) = &pagina.capa {
                validar_alt(capa, "capa.alt", &mut problemas);
            }
        }
        Conteudo::Post(post) => {
            validar_alt(&post.capa, "capa.alt", &mut problemas);
            if post.capa.largura < LARGURA_MINIMA_DA_CAPA {
                problemas.push(Problema::bloqueia(
                    "post.capa.pequena",
                    "capa",
                    format!(
                        "A imagem de capa tem {} px de largura. Envie uma com pelo menos {LARGURA_MINIMA_DA_CAPA} px para ela aparecer em destaque no Google.",
                        post.capa.largura
                    ),
                ));
            }
            if post.autor.bio.trim().is_empty() {
                problemas.push(Problema::bloqueia(
                    "post.autor.incompleto",
                    "autor.bio",
                    "Escreva a apresentação do autor. Quem lê quer saber quem assina o texto.",
                ));
            }
            if post.autor.foto.id.trim().is_empty() {
                problemas.push(Problema::bloqueia(
                    "post.autor.incompleto",
                    "autor.foto",
                    "Envie a foto do autor.",
                ));
            } else {
                validar_alt(&post.autor.foto, "autor.foto.alt", &mut problemas);
            }
        }
        Conteudo::Produto(produto) => {
            for (i, imagem) in produto.imagens.iter().enumerate() {
                validar_alt(imagem, &format!("imagens[{i}].alt"), &mut problemas);
            }
            if produto.variacoes.is_empty() {
                problemas.push(Problema::bloqueia(
                    "produto.sem-variacao",
                    "variacoes",
                    "Cadastre ao menos uma opção de venda, com preço.",
                ));
            }
            for (i, variacao) in produto.variacoes.iter().enumerate() {
                if variacao.preco_centavos <= 0 {
                    problemas.push(Problema::bloqueia(
                        "produto.preco",
                        format!("variacoes[{i}].precoCentavos"),
                        "Informe um preço maior que zero.",
                    ));
                }
            }
        }
    }

    if !documento.seo().indexar {
        problemas.push(Problema::avisa(
            "seo.noindex",
            "seo.indexar",
            "Esta página está escondida do Google e de outros buscadores. Ninguém vai encontrá-la pela busca.",
        ));
    }
    if let Some(peso) = contexto
        .peso_kb
        .filter(|peso| *peso > contexto.orcamento_peso_kb)
    {
        problemas.push(Problema::avisa(
            "pagina.peso",
            "",
            format!(
                "A página pesa {peso} kB e o recomendado é até {} kB. Páginas pesadas demoram a abrir no celular.",
                contexto.orcamento_peso_kb
            ),
        ));
    }

    problemas
}

fn validar_busca(documento: &Documento, contexto: &Contexto, problemas: &mut Vec<Problema>) {
    let seo = documento.seo();
    let igual = |a: &str, b: &str| a.trim().to_lowercase() == b.trim().to_lowercase();

    let titulo = seo.titulo.trim();
    if titulo.is_empty() {
        problemas.push(Problema::bloqueia(
            "seo.titulo.vazio",
            "seo.titulo",
            "Escreva o título que vai aparecer no Google.",
        ));
    } else {
        if contexto
            .titulos_em_uso
            .iter()
            .any(|outro| igual(outro, titulo))
        {
            problemas.push(Problema::bloqueia(
                "seo.titulo.repetido",
                "seo.titulo",
                "Outra página do site já usa este título. Cada página precisa do seu, para o Google saber qual mostrar.",
            ));
        }
        let tamanho = comprimento(titulo);
        if tamanho > TITULO_MAXIMO {
            problemas.push(Problema::bloqueia(
                "seo.titulo.excedido",
                "seo.titulo",
                format!(
                    "O título tem {tamanho} caracteres. Reduza para no máximo {TITULO_MAXIMO}."
                ),
            ));
        } else if tamanho > TITULO_RECOMENDADO {
            problemas.push(Problema::avisa(
                "seo.titulo.longo",
                "seo.titulo",
                format!("O título tem {tamanho} caracteres. Acima de {TITULO_RECOMENDADO} o Google costuma cortar o final."),
            ));
        }
    }

    let descricao = seo.descricao.trim();
    if descricao.is_empty() {
        problemas.push(Problema::bloqueia(
            "seo.descricao.vazia",
            "seo.descricao",
            "Escreva a descrição que aparece embaixo do título no Google.",
        ));
    } else {
        if contexto
            .descricoes_em_uso
            .iter()
            .any(|outra| igual(outra, descricao))
        {
            problemas.push(Problema::bloqueia(
                "seo.descricao.repetida",
                "seo.descricao",
                "Outra página do site já usa esta descrição. Escreva uma própria para esta página.",
            ));
        }
        let tamanho = comprimento(descricao);
        if tamanho > DESCRICAO_MAXIMA {
            problemas.push(Problema::bloqueia(
                "seo.descricao.excedida",
                "seo.descricao",
                format!("A descrição tem {tamanho} caracteres. Reduza para no máximo {DESCRICAO_MAXIMA}."),
            ));
        } else if tamanho > DESCRICAO_RECOMENDADA {
            problemas.push(Problema::avisa(
                "seo.descricao.longa",
                "seo.descricao",
                format!(
                    "A descrição tem {tamanho} caracteres. Acima de {DESCRICAO_RECOMENDADA} o Google costuma cortar o final."
                ),
            ));
        }
    }
}

fn validar_slug(documento: &Documento, contexto: &Contexto, problemas: &mut Vec<Problema>) {
    let slug = documento.slug();
    let eh_pagina = matches!(documento.conteudo, Conteudo::Pagina(_));

    if slug.is_empty() {
        // Só a home, que é uma página, pode não ter endereço próprio.
        if !eh_pagina {
            problemas.push(Problema::bloqueia(
                "slug.vazio",
                "slug",
                "Defina o endereço desta página.",
            ));
        }
    } else if normalizar_slug(slug).is_empty() {
        problemas.push(Problema::bloqueia(
            "slug.vazio",
            "slug",
            "O endereço precisa ter letras ou números.",
        ));
    } else if normalizar_slug(slug) != slug {
        problemas.push(Problema::bloqueia(
            "slug.formato",
            "slug",
            format!(
                "O endereço só pode ter letras minúsculas, números e hífen. Use \"{}\".",
                normalizar_slug(slug)
            ),
        ));
    }

    if let Some(anterior) = contexto.slug_publicado {
        if anterior != slug && !contexto.redirecionados.iter().any(|r| r == anterior) {
            problemas.push(Problema::bloqueia(
                "slug.sem-redirecionamento",
                "slug",
                "Esta página já está no ar com outro endereço. Crie o redirecionamento do endereço antigo para quem tem o link não cair em página inexistente.",
            ));
        }
    }
}

fn validar_corpo(blocos: &[Bloco], problemas: &mut Vec<Problema>) {
    // O título da página é o nível 1; o corpo começa no 2.
    let mut nivel_anterior = 1u8;
    for (i, bloco) in blocos.iter().enumerate() {
        match bloco {
            Bloco::Titulo { nivel, .. } => {
                if !(2..=4).contains(nivel) || *nivel > nivel_anterior + 1 {
                    problemas.push(Problema::bloqueia(
                        "corpo.titulo.nivel",
                        format!("corpo[{i}].nivel"),
                        format!(
                            "Este título pula um nível. Depois de um título de nível {nivel_anterior} vem um de nível {}, no máximo.",
                            nivel_anterior + 1
                        ),
                    ));
                }
                nivel_anterior = (*nivel).clamp(2, 4);
            }
            Bloco::Imagem { midia } => {
                validar_alt(midia, &format!("corpo[{i}].midia.alt"), problemas)
            }
            Bloco::Paragrafo { trechos } => {
                for (j, trecho) in trechos.iter().enumerate() {
                    if trecho
                        .link
                        .as_deref()
                        .is_some_and(|link| !link_valido(link))
                    {
                        problemas.push(link_invalido(format!("corpo[{i}].trechos[{j}].link")));
                    }
                }
            }
            Bloco::Chamada { url, .. } => {
                if !link_valido(url) {
                    problemas.push(link_invalido(format!("corpo[{i}].url")));
                }
            }
            Bloco::Video { url, .. } => {
                if incorporacao_de_video(url).is_none() {
                    problemas.push(Problema::bloqueia(
                        "video.origem",
                        format!("corpo[{i}].url"),
                        "Use o endereço de um vídeo do YouTube ou do Vimeo.",
                    ));
                }
            }
            Bloco::Lista { .. }
            | Bloco::Citacao { .. }
            | Bloco::Tabela { .. }
            | Bloco::Perguntas { .. } => {}
        }
    }
}

fn link_invalido(campo: String) -> Problema {
    Problema::bloqueia(
        "link.invalido",
        campo,
        "Este link não é válido. Use um endereço que comece com https:// ou um caminho do próprio site, como /contato.",
    )
}

fn validar_alt(midia: &Midia, campo: &str, problemas: &mut Vec<Problema>) {
    let alt = midia.alt.trim();
    if alt.is_empty() {
        problemas.push(Problema::bloqueia(
            "midia.alt.vazio",
            campo,
            "Escreva uma descrição da foto para quem não enxerga a imagem.",
        ));
    } else if parece_nome_de_arquivo(alt, midia) {
        problemas.push(Problema::bloqueia(
            "midia.alt.arquivo",
            campo,
            "A descrição da foto é o nome do arquivo. Descreva o que aparece na imagem.",
        ));
    }
}

fn parece_nome_de_arquivo(alt: &str, midia: &Midia) -> bool {
    let minusculo = alt.to_lowercase();
    if [".jpg", ".jpeg", ".png", ".webp", ".avif", ".gif"]
        .iter()
        .any(|extensao| minusculo.ends_with(extensao))
    {
        return true;
    }
    let alt = normalizar_slug(alt);
    midia.variantes.iter().any(|variante| {
        let nome = nome_base(&variante.url);
        !nome.is_empty() && nome == alt
    })
}

/// O nome original do arquivo, deduzido da URL da variante: sem pasta, sem
/// extensão, sem largura e sem o trecho do hash.
fn nome_base(url: &str) -> String {
    let arquivo = url
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("");
    let sem_extensao = arquivo.rsplit_once('.').map_or(arquivo, |(nome, _)| nome);
    let mut partes: Vec<&str> = sem_extensao.split('-').collect();
    if partes
        .last()
        .is_some_and(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    {
        partes.pop();
    }
    if partes
        .last()
        .is_some_and(|p| p.len() == 8 && p.chars().all(|c| c.is_ascii_hexdigit()))
    {
        partes.pop();
    }
    normalizar_slug(&partes.join("-"))
}

/// Formatos de arquivo aceitos no envio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatoDeEnvio {
    Jpeg,
    Png,
    Webp,
}

/// Lê o formato dos primeiros bytes, não da extensão.
pub fn formato_do_arquivo(bytes: &[u8]) -> Option<FormatoDeEnvio> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(FormatoDeEnvio::Jpeg)
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some(FormatoDeEnvio::Png)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(FormatoDeEnvio::Webp)
    } else {
        None
    }
}

pub fn validar_upload(nome: &str, bytes: &[u8]) -> Vec<Problema> {
    let mut problemas = Vec::new();
    if bytes.len() > UPLOAD_MAXIMO_BYTES {
        problemas.push(Problema::bloqueia(
            "upload.grande",
            nome,
            format!(
                "O arquivo tem {} MB. O limite é {} MB.",
                bytes.len().div_ceil(1024 * 1024),
                UPLOAD_MAXIMO_BYTES / (1024 * 1024)
            ),
        ));
    }
    if formato_do_arquivo(bytes).is_none() {
        problemas.push(Problema::bloqueia(
            "upload.formato",
            nome,
            "Envie a imagem em JPG, PNG ou WebP.",
        ));
    }
    problemas
}

/// O documento não passaria na validação sem contexto. É falha de quem
/// chamou o gerador, não de quem editou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentoInvalido {
    pub caminho: String,
    pub codigo: &'static str,
    pub campo: String,
}

impl fmt::Display for DocumentoInvalido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "documento {} não pode ser publicado: {} em \"{}\"",
            self.caminho, self.codigo, self.campo
        )
    }
}

impl std::error::Error for DocumentoInvalido {}

pub(crate) fn exigir_publicavel(documento: &Documento) -> Result<(), DocumentoInvalido> {
    match validar(documento, &Contexto::vazio())
        .into_iter()
        .find(|p| p.gravidade == Gravidade::Bloqueia)
    {
        Some(problema) => Err(DocumentoInvalido {
            caminho: documento.caminho.clone(),
            codigo: problema.codigo,
            campo: problema.campo,
        }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::demonstracao;
    use crate::tipos::{Trecho, Variacao};

    fn codigos(problemas: &[Problema]) -> Vec<&'static str> {
        problemas.iter().map(|p| p.codigo).collect()
    }

    fn validar_sem_contexto(documento: &Documento) -> Vec<Problema> {
        validar(documento, &Contexto::vazio())
    }

    fn com_post(altera: impl FnOnce(&mut crate::tipos::Post)) -> Documento {
        let mut documento = demonstracao::post_de_exemplo();
        if let Conteudo::Post(post) = &mut documento.conteudo {
            altera(post);
        }
        documento
    }

    fn com_produto(altera: impl FnOnce(&mut crate::tipos::Product)) -> Documento {
        let mut documento = demonstracao::produto_de_exemplo();
        if let Conteudo::Produto(produto) = &mut documento.conteudo {
            altera(produto);
        }
        documento
    }

    #[test]
    fn exemplos_passam_limpos() {
        assert_eq!(
            validar_sem_contexto(&demonstracao::post_de_exemplo()),
            vec![]
        );
        assert_eq!(
            validar_sem_contexto(&demonstracao::produto_de_exemplo()),
            vec![]
        );
    }

    #[test]
    fn slug_perde_acento_caixa_e_sobra() {
        assert_eq!(
            normalizar_slug("  Ação & Reação: 2ª edição!  "),
            "acao-reacao-2a-edicao"
        );
        assert_eq!(
            normalizar_slug("São_João--da__Boa Vista"),
            "sao-joao-da-boa-vista"
        );
        assert_eq!(normalizar_slug("!!!"), "");
    }

    #[test]
    fn comprimento_conta_o_que_a_pessoa_ve() {
        assert_eq!(comprimento("Ação"), 4);
        // "a" seguido de til combinante é uma letra só.
        assert_eq!(comprimento("a\u{0303}o"), 2);
        assert_eq!(comprimento("👨‍👩‍👧 família"), 9);
    }

    #[test]
    fn titulo_vazio_longo_e_excedido() {
        let vazio = com_post(|p| p.seo.titulo = "  ".into());
        assert!(codigos(&validar_sem_contexto(&vazio)).contains(&"seo.titulo.vazio"));

        let no_limite = com_post(|p| p.seo.titulo = "ã".repeat(60));
        assert_eq!(validar_sem_contexto(&no_limite), vec![]);

        let longo = com_post(|p| p.seo.titulo = "ã".repeat(61));
        let problemas = validar_sem_contexto(&longo);
        assert_eq!(codigos(&problemas), vec!["seo.titulo.longo"]);
        assert!(pode_publicar(&problemas));

        let excedido = com_post(|p| p.seo.titulo = "ã".repeat(71));
        let problemas = validar_sem_contexto(&excedido);
        assert_eq!(codigos(&problemas), vec!["seo.titulo.excedido"]);
        assert!(!pode_publicar(&problemas));
    }

    #[test]
    fn descricao_vazia_longa_e_excedida() {
        let vazia = com_post(|p| p.seo.descricao = String::new());
        assert!(codigos(&validar_sem_contexto(&vazia)).contains(&"seo.descricao.vazia"));

        let longa = com_post(|p| p.seo.descricao = "d".repeat(156));
        assert_eq!(
            codigos(&validar_sem_contexto(&longa)),
            vec!["seo.descricao.longa"]
        );

        let excedida = com_post(|p| p.seo.descricao = "d".repeat(171));
        assert_eq!(
            codigos(&validar_sem_contexto(&excedida)),
            vec!["seo.descricao.excedida"]
        );
    }

    #[test]
    fn titulo_e_descricao_repetidos_no_site() {
        let documento = demonstracao::post_de_exemplo();
        let titulos = [format!("  {}  ", documento.seo().titulo.to_uppercase())];
        let descricoes = [documento.seo().descricao.clone()];
        let contexto = Contexto {
            titulos_em_uso: &titulos,
            descricoes_em_uso: &descricoes,
            ..Contexto::vazio()
        };
        assert_eq!(
            codigos(&validar(&documento, &contexto)),
            vec!["seo.titulo.repetido", "seo.descricao.repetida"]
        );
    }

    #[test]
    fn slug_vazio_fora_do_formato_e_sem_redirecionamento() {
        let vazio = com_post(|p| p.slug = String::new());
        assert!(codigos(&validar_sem_contexto(&vazio)).contains(&"slug.vazio"));

        let so_simbolos = com_post(|p| p.slug = "!!!".into());
        assert!(codigos(&validar_sem_contexto(&so_simbolos)).contains(&"slug.vazio"));

        let com_acento = com_post(|p| p.slug = "Promoção".into());
        assert_eq!(
            codigos(&validar_sem_contexto(&com_acento)),
            vec!["slug.formato"]
        );

        let documento = demonstracao::post_de_exemplo();
        let sem = Contexto {
            slug_publicado: Some("endereco-antigo"),
            ..Contexto::vazio()
        };
        assert_eq!(
            codigos(&validar(&documento, &sem)),
            vec!["slug.sem-redirecionamento"]
        );

        let redirecionados = ["endereco-antigo".to_string()];
        let com = Contexto {
            slug_publicado: Some("endereco-antigo"),
            redirecionados: &redirecionados,
            ..Contexto::vazio()
        };
        assert_eq!(validar(&documento, &com), vec![]);
    }

    #[test]
    fn home_pode_ter_slug_vazio() {
        let home = demonstracao::documentos()
            .into_iter()
            .find(Documento::eh_home)
            .expect("a fixture tem home");
        assert_eq!(validar_sem_contexto(&home), vec![]);
    }

    #[test]
    fn titulo_de_secao_nao_pula_nivel() {
        let titulo = |nivel: u8| Bloco::Titulo {
            nivel,
            texto: "Seção".into(),
        };

        let certo =
            com_post(|p| p.corpo = vec![titulo(2), titulo(3), titulo(4), titulo(2), titulo(3)]);
        assert_eq!(validar_sem_contexto(&certo), vec![]);

        let comeca_no_tres = com_post(|p| p.corpo = vec![titulo(3)]);
        assert_eq!(
            codigos(&validar_sem_contexto(&comeca_no_tres)),
            vec!["corpo.titulo.nivel"]
        );

        let pula = com_post(|p| p.corpo = vec![titulo(2), titulo(4)]);
        let problemas = validar_sem_contexto(&pula);
        assert_eq!(codigos(&problemas), vec!["corpo.titulo.nivel"]);
        assert_eq!(problemas[0].campo, "corpo[1].nivel");

        let nivel_um = com_post(|p| p.corpo = vec![titulo(1)]);
        assert_eq!(
            codigos(&validar_sem_contexto(&nivel_um)),
            vec!["corpo.titulo.nivel"]
        );
    }

    #[test]
    fn imagem_precisa_de_descricao_de_verdade() {
        let sem_alt = com_post(|p| p.capa.alt = " ".into());
        let problemas = validar_sem_contexto(&sem_alt);
        assert_eq!(codigos(&problemas), vec!["midia.alt.vazio"]);
        assert_eq!(problemas[0].campo, "capa.alt");

        let extensao = com_post(|p| p.capa.alt = "IMG_2041.JPG".into());
        assert_eq!(
            codigos(&validar_sem_contexto(&extensao)),
            vec!["midia.alt.arquivo"]
        );

        // A URL da variante é `/midia/<nome>-<hash>-<largura>.<formato>`.
        let nome_do_arquivo = com_post(|p| {
            let nome = nome_base(&p.capa.variantes[0].url);
            p.capa.alt = nome.replace('-', " ");
        });
        assert_eq!(
            codigos(&validar_sem_contexto(&nome_do_arquivo)),
            vec!["midia.alt.arquivo"]
        );
    }

    #[test]
    fn nome_base_desconta_hash_e_largura() {
        assert_eq!(
            nome_base("/midia/bancada-de-marcenaria-1a2b3c4d-1200.webp?v=2"),
            "bancada-de-marcenaria"
        );
        assert_eq!(nome_base("https://x.com/a/foto.avif"), "foto");
    }

    #[test]
    fn post_exige_autor_completo_e_capa_grande() {
        let sem_bio = com_post(|p| p.autor.bio = String::new());
        assert_eq!(
            codigos(&validar_sem_contexto(&sem_bio)),
            vec!["post.autor.incompleto"]
        );

        let sem_foto = com_post(|p| p.autor.foto.id = String::new());
        assert_eq!(
            codigos(&validar_sem_contexto(&sem_foto)),
            vec!["post.autor.incompleto"]
        );

        let capa_pequena = com_post(|p| p.capa.largura = 1199);
        assert_eq!(
            codigos(&validar_sem_contexto(&capa_pequena)),
            vec!["post.capa.pequena"]
        );
    }

    #[test]
    fn link_e_video_so_de_origem_aceita() {
        let link = |destino: &str| Bloco::Paragrafo {
            trechos: vec![Trecho {
                texto: "clique".into(),
                link: Some(destino.into()),
                ..Trecho::default()
            }],
        };
        let ruim = com_post(|p| {
            p.corpo = vec![
                link("javascript:alert(1)"),
                Bloco::Chamada {
                    texto: "Fale".into(),
                    rotulo: "Contato".into(),
                    url: "//golpe.com".into(),
                },
                Bloco::Video {
                    url: "https://exemplo.com/v.mp4".into(),
                    titulo: "Vídeo".into(),
                },
            ]
        });
        assert_eq!(
            codigos(&validar_sem_contexto(&ruim)),
            vec!["link.invalido", "link.invalido", "video.origem"]
        );

        let bom = com_post(|p| {
            p.corpo = vec![
                link("/contato"),
                link("https://exemplo.com.br"),
                Bloco::Video {
                    url: "https://vimeo.com/123".into(),
                    titulo: "Vídeo".into(),
                },
            ]
        });
        assert_eq!(validar_sem_contexto(&bom), vec![]);
    }

    #[test]
    fn produto_exige_variacao_com_preco() {
        let sem = com_produto(|p| p.variacoes.clear());
        assert_eq!(
            codigos(&validar_sem_contexto(&sem)),
            vec!["produto.sem-variacao"]
        );

        let gratis = com_produto(|p| {
            p.variacoes = vec![
                Variacao {
                    preco_centavos: 0,
                    ..p.variacoes[0].clone()
                },
                Variacao {
                    preco_centavos: -100,
                    ..p.variacoes[0].clone()
                },
            ]
        });
        let problemas = validar_sem_contexto(&gratis);
        assert_eq!(codigos(&problemas), vec!["produto.preco", "produto.preco"]);
        assert_eq!(problemas[1].campo, "variacoes[1].precoCentavos");
    }

    #[test]
    fn noindex_e_peso_avisam_sem_barrar() {
        let escondido = com_post(|p| p.seo.indexar = false);
        let contexto = Contexto {
            peso_kb: Some(900),
            orcamento_peso_kb: 500,
            ..Contexto::vazio()
        };
        let problemas = validar(&escondido, &contexto);
        assert_eq!(codigos(&problemas), vec!["seo.noindex", "pagina.peso"]);
        assert!(pode_publicar(&problemas));

        let dentro = Contexto {
            peso_kb: Some(500),
            orcamento_peso_kb: 500,
            ..Contexto::vazio()
        };
        assert_eq!(validar(&demonstracao::post_de_exemplo(), &dentro), vec![]);
    }

    #[test]
    fn data_so_muda_quando_o_conteudo_muda() {
        let antes = demonstracao::post_de_exemplo();
        assert!(!deve_atualizar_data(&antes, &antes.clone()));

        let so_busca = com_post(|p| p.seo.descricao = "Outra descrição para a busca.".into());
        assert!(!deve_atualizar_data(&antes, &so_busca));

        let titulo = com_post(|p| p.titulo.push('!'));
        assert!(deve_atualizar_data(&antes, &titulo));

        let corpo = com_post(|p| {
            p.corpo.push(Bloco::Titulo {
                nivel: 2,
                texto: "Novo".into(),
            })
        });
        assert!(deve_atualizar_data(&antes, &corpo));
    }

    #[test]
    fn upload_le_o_formato_dos_bytes() {
        assert_eq!(
            validar_upload("foto.png", &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]),
            vec![]
        );
        assert_eq!(
            validar_upload("a.png", &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            vec![]
        );
        assert_eq!(validar_upload("a.webp", b"RIFF\0\0\0\0WEBPVP8 "), vec![]);

        // Extensão de imagem com conteúdo de outra coisa.
        assert_eq!(
            codigos(&validar_upload("foto.jpg", b"<svg onload=alert(1)>")),
            vec!["upload.formato"]
        );
        assert_eq!(
            codigos(&validar_upload("vazio.jpg", &[])),
            vec!["upload.formato"]
        );
    }

    #[test]
    fn upload_grande_e_recusado() {
        let mut bytes = vec![0u8; UPLOAD_MAXIMO_BYTES + 1];
        bytes[..3].copy_from_slice(&[0xFF, 0xD8, 0xFF]);
        assert_eq!(
            codigos(&validar_upload("grande.jpg", &bytes)),
            vec!["upload.grande"]
        );
    }

    #[test]
    fn gerador_recusa_documento_que_nao_pode_ser_publicado() {
        let ruim = com_post(|p| p.capa.alt = String::new());
        let erro = exigir_publicavel(&ruim).expect_err("capa sem descrição");
        assert_eq!(erro.codigo, "midia.alt.vazio");
        assert_eq!(erro.campo, "capa.alt");
        assert!(erro.to_string().contains(&ruim.caminho));
    }
}
