//! O cabeçalho da página: título, metas, canônica, Open Graph e JSON-LD.

use chrono::SecondsFormat;
use serde::Serialize;
use serde_json::Value;

use crate::escape::{json_para_script, marcacao};
use crate::midia::variante_principal;
use crate::schema::montar_json_ld;
use crate::tipos::{Conteudo, Documento, Formato, Midia, Site, Variante};
use crate::url::endereco_absoluto_valido;
use crate::validacao::DocumentoInvalido;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AtributoMeta {
    Name,
    Property,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Meta {
    pub atributo: AtributoMeta,
    pub chave: String,
    pub conteudo: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Link {
    pub rel: String,
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cabecalho {
    pub titulo: String,
    pub metas: Vec<Meta>,
    pub links: Vec<Link>,
    pub json_ld: Vec<Value>,
}

/// O endereço oficial de um caminho: origem mais caminho, sem query, sem
/// fragmento e sem barra final, exceto na home.
pub fn canonica(site: &Site, caminho: &str) -> String {
    let caminho = caminho
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .trim_matches('/');
    if caminho.is_empty() {
        format!("{}/", site.origem_limpa())
    } else {
        format!("{}/{caminho}", site.origem_limpa())
    }
}

/// Página na busca, com a imagem grande e o trecho inteiro liberados: é o que
/// a busca de imagens e o Discover pedem para mostrar a foto em destaque.
const INDEXAR: &str =
    "index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1";

pub fn montar_cabecalho(
    documento: &Documento,
    site: &Site,
) -> Result<Cabecalho, DocumentoInvalido> {
    // Valida uma vez: `montar_json_ld` recusa o documento que não pode ser publicado.
    let json_ld = montar_json_ld(documento, site)?;
    let seo = documento.seo();
    // Canônica para fora só sai se for válida; a validação já barra a outra.
    let endereco = seo
        .canonica
        .as_deref()
        .map(str::trim)
        .filter(|outra| endereco_absoluto_valido(outra))
        .map_or_else(|| canonica(site, &documento.caminho), String::from);

    let mut metas = Vec::new();
    let mut nome = |chave: &str, conteudo: &str| {
        metas.push(Meta {
            atributo: AtributoMeta::Name,
            chave: chave.into(),
            conteudo: conteudo.into(),
        })
    };
    nome("description", &seo.descricao);
    nome(
        "robots",
        if seo.indexar {
            INDEXAR
        } else {
            "noindex, follow"
        },
    );

    let imagem = seo
        .imagem_social
        .as_ref()
        .or_else(|| documento.capa())
        .and_then(|midia| imagem_social(midia, site));
    nome(
        "twitter:card",
        if imagem.is_some() {
            "summary_large_image"
        } else {
            "summary"
        },
    );
    nome("twitter:title", &seo.titulo);
    nome("twitter:description", &seo.descricao);
    if let Some(imagem) = &imagem {
        nome("twitter:image", &imagem.url);
    }

    let mut propriedade = |chave: &str, conteudo: &str| {
        metas.push(Meta {
            atributo: AtributoMeta::Property,
            chave: chave.into(),
            conteudo: conteudo.into(),
        })
    };
    let tipo = match &documento.conteudo {
        // O produto de catálogo não tem preço: `product` pediria um.
        Conteudo::Pagina(_) | Conteudo::Item(_) => "website",
        Conteudo::Post(_) => "article",
        Conteudo::Produto(_) => "product",
    };
    propriedade("og:type", tipo);
    propriedade("og:site_name", &site.nome);
    propriedade("og:locale", &site.idioma.replace('-', "_"));
    propriedade("og:title", &seo.titulo);
    propriedade("og:description", &seo.descricao);
    propriedade("og:url", &endereco);
    if let Some(imagem) = &imagem {
        propriedade("og:image", &imagem.url);
        propriedade("og:image:width", &imagem.largura.to_string());
        propriedade("og:image:height", &imagem.altura.to_string());
        propriedade("og:image:type", imagem.tipo);
        propriedade("og:image:alt", &imagem.alt);
    }
    if let Conteudo::Post(post) = &documento.conteudo {
        propriedade(
            "article:published_time",
            &post.publicado_em.to_rfc3339_opts(SecondsFormat::Secs, true),
        );
        propriedade(
            "article:modified_time",
            &post
                .atualizado_em
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        );
    }

    Ok(Cabecalho {
        titulo: seo.titulo.clone(),
        metas,
        links: vec![Link {
            rel: "canonical".into(),
            href: endereco,
        }],
        json_ld,
    })
}

struct ImagemSocial {
    url: String,
    tipo: &'static str,
    largura: u32,
    altura: u32,
    alt: String,
}

/// A extensão do JPEG que quem consome serve ao lado de cada WebP, com o
/// mesmo nome: WhatsApp, Telegram e outros leitores de prévia não abrem WebP.
pub const EXTENSAO_SOCIAL: &str = "jpg";

/// O endereço que vai no `og:image`: o JPEG irmão do WebP. Outro formato
/// segue como está.
pub fn url_social(variante: &Variante) -> (String, &'static str) {
    match (variante.formato, variante.url.strip_suffix(".webp")) {
        (Formato::Webp, Some(nome)) => (format!("{nome}.{EXTENSAO_SOCIAL}"), "image/jpeg"),
        _ => (variante.url.clone(), variante.formato.tipo_mime()),
    }
}

fn imagem_social(midia: &Midia, site: &Site) -> Option<ImagemSocial> {
    let variante = variante_principal(midia)?;
    let (url, tipo) = url_social(variante);
    // A variante guarda só a largura; a altura segue a proporção do original.
    let altura = if midia.largura == 0 {
        0
    } else {
        (u64::from(midia.altura) * u64::from(variante.largura) / u64::from(midia.largura)) as u32
    };
    Some(ImagemSocial {
        url: site.url_absoluta(&url),
        tipo,
        largura: variante.largura,
        altura,
        alt: midia.alt.clone(),
    })
}

impl Cabecalho {
    /// O conteúdo do `<head>` que depende do documento, uma tag por linha.
    pub fn para_html(&self) -> String {
        let mut linhas = vec![format!("<title>{}</title>", marcacao(&self.titulo))];
        for meta in &self.metas {
            let atributo = match meta.atributo {
                AtributoMeta::Name => "name",
                AtributoMeta::Property => "property",
            };
            linhas.push(format!(
                r#"<meta {atributo}="{}" content="{}">"#,
                marcacao(&meta.chave),
                marcacao(&meta.conteudo)
            ));
        }
        for link in &self.links {
            linhas.push(format!(
                r#"<link rel="{}" href="{}">"#,
                marcacao(&link.rel),
                marcacao(&link.href)
            ));
        }
        for item in &self.json_ld {
            linhas.push(format!(
                r#"<script type="application/ld+json">{}</script>"#,
                json_para_script(item)
            ));
        }
        linhas.join("\n")
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::demonstracao;

    fn meta<'a>(cabecalho: &'a Cabecalho, chave: &str) -> Option<&'a str> {
        cabecalho
            .metas
            .iter()
            .find(|m| m.chave == chave)
            .map(|m| m.conteudo.as_str())
    }

    fn post(altera: impl FnOnce(&mut crate::tipos::Post)) -> Documento {
        let mut documento = demonstracao::post_de_exemplo();
        if let Conteudo::Post(post) = &mut documento.conteudo {
            altera(post);
        }
        documento
    }

    #[test]
    fn canonica_limpa_query_fragmento_e_barra() {
        let site = demonstracao::site();
        let origem = site.origem.clone();
        assert_eq!(canonica(&site, "/"), format!("{origem}/"));
        assert_eq!(canonica(&site, ""), format!("{origem}/"));
        assert_eq!(
            canonica(&site, "/blog/post/?utm_source=x#topo"),
            format!("{origem}/blog/post")
        );
        assert_eq!(canonica(&site, "contato"), format!("{origem}/contato"));
    }

    #[test]
    fn canonica_para_fora_vale_no_link_e_no_og() {
        let site = demonstracao::site();
        let mut documento = demonstracao::post_de_exemplo();
        if let Conteudo::Post(post) = &mut documento.conteudo {
            post.seo.canonica = Some(" https://exemplo.example/oficial ".into());
        }
        let cabecalho = montar_cabecalho(&documento, &site).expect("fixture válida");
        assert_eq!(cabecalho.links[0].href, "https://exemplo.example/oficial");
        assert_eq!(
            meta(&cabecalho, "og:url"),
            Some("https://exemplo.example/oficial")
        );
    }

    #[test]
    fn post_tem_canonica_og_e_datas() {
        let site = demonstracao::site();
        let documento = demonstracao::post_de_exemplo();
        let cabecalho = montar_cabecalho(&documento, &site).expect("fixture válida");

        assert_eq!(
            cabecalho.links,
            vec![Link {
                rel: "canonical".into(),
                href: canonica(&site, &documento.caminho)
            }]
        );
        assert_eq!(meta(&cabecalho, "robots"), Some(INDEXAR));
        assert_eq!(meta(&cabecalho, "og:type"), Some("article"));
        assert_eq!(meta(&cabecalho, "og:locale"), Some("pt_BR"));
        assert_eq!(
            meta(&cabecalho, "twitter:card"),
            Some("summary_large_image")
        );
        assert!(meta(&cabecalho, "og:image").is_some_and(|url| url.starts_with(&site.origem)));
        assert_eq!(meta(&cabecalho, "og:image:width"), Some("1200"));
        assert_eq!(meta(&cabecalho, "og:image:type"), Some("image/jpeg"));
        let imagem = meta(&cabecalho, "og:image").expect("og:image");
        assert!(imagem.ends_with("-1200.jpg"), "{imagem}");
        assert_eq!(meta(&cabecalho, "twitter:image"), Some(imagem));
        assert!(meta(&cabecalho, "article:published_time").is_some_and(|d| d.ends_with('Z')));
    }

    #[test]
    fn imagem_social_e_o_jpeg_irmao_do_webp() {
        let webp = Variante {
            formato: Formato::Webp,
            largura: 1024,
            url: "/m/icone-1a2b3c4d-1024.webp".into(),
        };
        assert_eq!(
            url_social(&webp),
            ("/m/icone-1a2b3c4d-1024.jpg".into(), "image/jpeg")
        );
        // Sem WebP, o formato que houver segue como está, com o tipo dele.
        let avif = Variante {
            formato: Formato::Avif,
            largura: 1024,
            url: "/m/icone-1a2b3c4d-1024.avif".into(),
        };
        assert_eq!(
            url_social(&avif),
            ("/m/icone-1a2b3c4d-1024.avif".into(), "image/avif")
        );
    }

    #[test]
    fn noindex_vira_meta_robots() {
        let cabecalho = montar_cabecalho(&post(|p| p.seo.indexar = false), &demonstracao::site())
            .expect("válido");
        assert_eq!(meta(&cabecalho, "robots"), Some("noindex, follow"));
    }

    #[test]
    fn sem_imagem_o_card_e_resumido_e_nao_sobra_campo_vazio() {
        let home = demonstracao::documentos()
            .into_iter()
            .find(Documento::eh_home)
            .expect("home");
        let cabecalho = montar_cabecalho(&home, &demonstracao::site()).expect("válido");
        assert_eq!(meta(&cabecalho, "twitter:card"), Some("summary"));
        assert_eq!(meta(&cabecalho, "og:image"), None);
        assert!(cabecalho.metas.iter().all(|m| !m.conteudo.is_empty()));
    }

    #[test]
    fn texto_de_quem_edita_nao_escapa_do_head() {
        let malicioso = post(|p| {
            p.seo.titulo = r#"Mesa "sob medida" </title><script>alert(1)</script>"#.into();
            p.titulo = "</script><script>alert(2)</script>".into();
        });
        let html = montar_cabecalho(&malicioso, &demonstracao::site())
            .expect("válido")
            .para_html();

        assert_eq!(html.matches("<title>").count(), 1);
        assert_eq!(html.matches("</title>").count(), 1);
        // Só os scripts de JSON-LD abrem e fecham.
        assert_eq!(
            html.matches("<script").count(),
            html.matches(r#"<script type="application/ld+json">"#)
                .count()
        );
        assert_eq!(
            html.matches("</script>").count(),
            html.matches("<script").count()
        );
        assert!(!html.contains("alert(1)</script>"));
    }

    #[test]
    fn json_ld_do_head_continua_sendo_json() {
        let html = montar_cabecalho(&demonstracao::post_de_exemplo(), &demonstracao::site())
            .expect("válido")
            .para_html();
        for linha in html.lines().filter(|l| l.starts_with("<script")) {
            let json = linha
                .trim_start_matches(r#"<script type="application/ld+json">"#)
                .trim_end_matches("</script>");
            assert!(serde_json::from_str::<Value>(json).is_ok(), "{json}");
        }
    }
}
