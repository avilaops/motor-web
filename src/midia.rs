//! HTML das imagens: `<picture>` com variantes, dimensões e prioridade.

use crate::escape::marcacao;
use crate::tipos::{Formato, Midia, Variante};

/// O papel da imagem na página decide como ela carrega.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Papel {
    /// Capa do post ou primeira imagem do produto. Uma por página.
    Principal,
    /// Qualquer outra. Carrega quando se aproxima da tela.
    Conteudo,
}

/// Largura que redes sociais e buscadores pedem para imagem de destaque.
const LARGURA_DE_DESTAQUE: u32 = 1200;

/// A variante usada como `src` de reserva, em Open Graph e em JSON-LD: a WebP
/// mais próxima de 1200 px.
pub fn url_principal(midia: &Midia) -> Option<&str> {
    variante_principal(midia).map(|v| v.url.as_str())
}

pub(crate) fn variante_principal(midia: &Midia) -> Option<&Variante> {
    // Empate de distância fica com a maior.
    let distancia = |v: &&Variante| {
        (
            v.largura.abs_diff(LARGURA_DE_DESTAQUE),
            u32::MAX - v.largura,
        )
    };
    midia
        .variantes
        .iter()
        .filter(|v| v.formato == Formato::Webp)
        .min_by_key(distancia)
        .or_else(|| midia.variantes.iter().min_by_key(distancia))
}

/// `<picture>` com AVIF, WebP e `<img>` de reserva. Devolve vazio quando a
/// mídia ainda não tem variante.
pub fn renderizar_midia(midia: &Midia, papel: Papel, sizes: &str) -> String {
    let Some(reserva) = url_principal(midia) else {
        return String::new();
    };

    let mut html = String::from("<picture>");
    for formato in [Formato::Avif, Formato::Webp] {
        let srcset = srcset(midia, formato);
        if !srcset.is_empty() {
            html.push_str(&format!(
                r#"<source type="{}" srcset="{}" sizes="{}">"#,
                formato.tipo_mime(),
                marcacao(&srcset),
                marcacao(sizes),
            ));
        }
    }
    let carregamento = match papel {
        Papel::Principal => r#"loading="eager" fetchpriority="high""#,
        Papel::Conteudo => r#"loading="lazy""#,
    };
    html.push_str(&format!(
        r#"<img src="{}" alt="{}" width="{}" height="{}" {} decoding="async">"#,
        marcacao(reserva),
        marcacao(&midia.alt),
        midia.largura,
        midia.altura,
        carregamento,
    ));
    html.push_str("</picture>");

    match legenda(midia) {
        Some(texto) => format!(
            "<figure>{html}<figcaption>{}</figcaption></figure>",
            marcacao(&texto)
        ),
        None => html,
    }
}

fn srcset(midia: &Midia, formato: Formato) -> String {
    let mut variantes: Vec<&Variante> = midia
        .variantes
        .iter()
        .filter(|v| v.formato == formato)
        .collect();
    variantes.sort_by_key(|v| v.largura);
    variantes
        .iter()
        .map(|v| format!("{} {}w", v.url, v.largura))
        .collect::<Vec<_>>()
        .join(", ")
}

fn legenda(midia: &Midia) -> Option<String> {
    let preenchido = |texto: &Option<String>| {
        texto
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(String::from)
    };
    // Sem crédito escrito, quem aparece é quem fez a imagem.
    let credito = if midia.ocultar_credito {
        None
    } else {
        preenchido(&midia.credito).or_else(|| preenchido(&midia.direitos.autoria))
    };
    match (preenchido(&midia.legenda), credito) {
        (Some(legenda), Some(credito)) => Some(format!("{legenda} Foto: {credito}")),
        (Some(legenda), None) => Some(legenda),
        (None, Some(credito)) => Some(format!("Foto: {credito}")),
        (None, None) => None,
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::tipos::Direitos;

    #[test]
    fn sem_credito_a_legenda_mostra_quem_fez_a_imagem() {
        let mut midia = midia();
        midia.direitos.autoria = Some("Helena Prado".into());
        assert!(
            renderizar_midia(&midia, Papel::Conteudo, "100vw")
                .contains("<figcaption>Foto: Helena Prado</figcaption>")
        );
        midia.credito = Some("Oficina Exemplo".into());
        assert!(
            renderizar_midia(&midia, Papel::Conteudo, "100vw")
                .contains("<figcaption>Foto: Oficina Exemplo</figcaption>")
        );
    }

    #[test]
    fn credito_oculto_nao_vira_legenda() {
        let mut midia = midia();
        midia.credito = Some("Oficina Exemplo".into());
        midia.ocultar_credito = true;
        assert!(!renderizar_midia(&midia, Papel::Conteudo, "100vw").contains("Foto:"));
        midia.legenda = Some("Logo da oficina.".into());
        assert!(
            renderizar_midia(&midia, Papel::Conteudo, "100vw")
                .contains("<figcaption>Logo da oficina.</figcaption>")
        );
    }

    fn midia() -> Midia {
        let variante = |formato: Formato, largura: u32| Variante {
            formato,
            largura,
            url: format!("/m/oficina-1a2b3c4d-{largura}.{}", formato.extensao()),
        };
        Midia {
            id: "m1".into(),
            alt: "Bancada \"principal\" da oficina".into(),
            largura: 1920,
            altura: 1080,
            variantes: vec![
                variante(Formato::Webp, 1920),
                variante(Formato::Avif, 480),
                variante(Formato::Webp, 480),
                variante(Formato::Avif, 1200),
                variante(Formato::Webp, 1200),
            ],
            legenda: None,
            credito: None,
            ocultar_credito: false,
            direitos: Direitos::default(),
        }
    }

    #[test]
    fn principal_carrega_com_prioridade() {
        let html = renderizar_midia(&midia(), Papel::Principal, "100vw");
        assert!(html.contains(r#"fetchpriority="high""#));
        assert!(html.contains(r#"loading="eager""#));
        assert!(html.contains(r#"width="1920" height="1080""#));
    }

    #[test]
    fn conteudo_carrega_depois_e_sem_prioridade() {
        let html = renderizar_midia(&midia(), Papel::Conteudo, "100vw");
        assert!(html.contains(r#"loading="lazy""#));
        assert!(!html.contains("fetchpriority"));
    }

    #[test]
    fn avif_vem_antes_e_as_larguras_em_ordem() {
        let html = renderizar_midia(&midia(), Papel::Conteudo, "100vw");
        let avif = html.find("image/avif").expect("fonte AVIF");
        let webp = html.find("image/webp").expect("fonte WebP");
        assert!(avif < webp);
        assert!(
            html.contains("oficina-1a2b3c4d-480.webp 480w, /m/oficina-1a2b3c4d-1200.webp 1200w")
        );
    }

    #[test]
    fn alt_e_escapado() {
        let html = renderizar_midia(&midia(), Papel::Conteudo, "100vw");
        assert!(html.contains("alt=\"Bancada &quot;principal&quot; da oficina\""));
    }

    #[test]
    fn reserva_e_a_webp_mais_proxima_de_1200() {
        assert_eq!(
            url_principal(&midia()),
            Some("/m/oficina-1a2b3c4d-1200.webp")
        );
    }

    #[test]
    fn sem_variante_nao_gera_nada() {
        let mut sem = midia();
        sem.variantes.clear();
        assert_eq!(renderizar_midia(&sem, Papel::Conteudo, "100vw"), "");
    }

    #[test]
    fn legenda_e_credito_viram_figure() {
        let mut com = midia();
        com.legenda = Some("Vista da oficina.".into());
        com.credito = Some("Equipe <interna>".into());
        let html = renderizar_midia(&com, Papel::Conteudo, "100vw");
        assert!(html.starts_with("<figure><picture>"));
        assert!(html.ends_with(
            "<figcaption>Vista da oficina. Foto: Equipe &lt;interna&gt;</figcaption></figure>"
        ));
    }
}
