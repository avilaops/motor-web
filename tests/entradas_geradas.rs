//! Nenhuma função entra em pânico com o que quem edita consegue digitar, e o
//! texto digitado nunca vira marcação.

use motor_web::conteudo::{contar_palavras, renderizar_corpo, texto_do_corpo};
use motor_web::demonstracao;
use motor_web::descoberta::{ItensDoSite, OpcoesRobots, gerar_llms, gerar_robots, gerar_sitemaps};
use motor_web::seo::{canonica, montar_cabecalho};
use motor_web::tipos::{Bloco, Conteudo, Documento, Pergunta, Trecho};
use motor_web::validacao::{Contexto, comprimento, normalizar_slug, validar, validar_upload};
use proptest::prelude::*;

/// Texto qualquer, com peso extra no que costuma quebrar gerador ingênuo.
fn texto() -> impl Strategy<Value = String> {
    prop_oneof![
        any::<String>(),
        "[<>&\"'/ a-z]{0,40}",
        Just("</script><script>alert(1)</script>".to_string()),
        Just("\"><img src=x onerror=alert(1)>".to_string()),
        Just("javascript:alert(1)".to_string()),
        Just("linha\nUser-agent: *\nAllow: /".to_string()),
    ]
}

fn bloco() -> impl Strategy<Value = Bloco> {
    prop_oneof![
        (
            texto(),
            any::<bool>(),
            any::<bool>(),
            proptest::option::of(texto())
        )
            .prop_map(|(t, negrito, italico, link)| {
                Bloco::Paragrafo {
                    trechos: vec![Trecho {
                        texto: t,
                        negrito,
                        italico,
                        link,
                    }],
                }
            }),
        (any::<u8>(), texto()).prop_map(|(nivel, texto)| Bloco::Titulo { nivel, texto }),
        (any::<bool>(), proptest::collection::vec(texto(), 0..4))
            .prop_map(|(ordenada, itens)| Bloco::Lista { ordenada, itens }),
        (texto(), proptest::option::of(texto()))
            .prop_map(|(texto, fonte)| Bloco::Citacao { texto, fonte }),
        (
            proptest::collection::vec(texto(), 0..3),
            proptest::collection::vec(proptest::collection::vec(texto(), 0..3), 0..3)
        )
            .prop_map(|(cabecalho, linhas)| Bloco::Tabela { cabecalho, linhas }),
        (texto(), texto()).prop_map(|(pergunta, resposta)| Bloco::Perguntas {
            itens: vec![Pergunta { pergunta, resposta }]
        }),
        (texto(), texto()).prop_map(|(url, titulo)| Bloco::Video { url, titulo }),
        (texto(), texto(), texto()).prop_map(|(texto, rotulo, url)| Bloco::Chamada {
            texto,
            rotulo,
            url
        }),
    ]
}

fn documento_com(titulo: String, descricao: String, slug: String, corpo: Vec<Bloco>) -> Documento {
    let mut documento = demonstracao::post_de_exemplo();
    if let Conteudo::Post(post) = &mut documento.conteudo {
        post.seo.titulo = titulo.clone();
        post.seo.descricao = descricao;
        post.titulo = titulo;
        post.slug = slug;
        post.corpo = corpo;
    }
    documento
}

/// Tags que o motor gera. Qualquer outra abertura de tag veio do texto.
const TAGS_DO_MOTOR: [&str; 24] = [
    "p",
    "a",
    "strong",
    "em",
    "h2",
    "h3",
    "h4",
    "ul",
    "ol",
    "li",
    "blockquote",
    "footer",
    "table",
    "thead",
    "tbody",
    "tr",
    "th",
    "td",
    "dl",
    "dt",
    "dd",
    "iframe",
    "aside",
    "picture",
];

proptest! {
    #[test]
    fn slug_normalizado_e_estavel_e_so_tem_o_permitido(entrada in texto()) {
        let slug = normalizar_slug(&entrada);
        prop_assert!(slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
        prop_assert!(!slug.starts_with('-') && !slug.ends_with('-') && !slug.contains("--"));
        prop_assert_eq!(normalizar_slug(&slug), slug);
    }

    #[test]
    fn comprimento_nunca_passa_do_numero_de_caracteres(entrada in texto()) {
        prop_assert!(comprimento(&entrada) <= entrada.chars().count());
    }

    #[test]
    fn corpo_qualquer_nao_gera_tag_fora_da_lista(blocos in proptest::collection::vec(bloco(), 0..8)) {
        let html = renderizar_corpo(&blocos);
        for trecho in html.split('<').skip(1) {
            let nome: String = trecho.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            prop_assert!(TAGS_DO_MOTOR.contains(&nome.as_str()), "tag inesperada <{}> em {}", nome, html);
        }
        // Como texto visível é inofensivo; como destino de link, não.
        for atributo in ["href=\"", "src=\""] {
            for trecho in html.split(atributo).skip(1) {
                let destino = trecho.split('"').next().unwrap_or("");
                prop_assert!(destino.starts_with('/') || destino.starts_with("http"), "destino {} em {}", destino, html);
            }
        }
        let _ = texto_do_corpo(&blocos);
        let _ = contar_palavras(&blocos);
    }

    #[test]
    fn documento_qualquer_valida_sem_panico_e_o_head_nao_vaza(
        titulo in texto(),
        descricao in texto(),
        slug in texto(),
        corpo in proptest::collection::vec(bloco(), 0..6),
    ) {
        let site = demonstracao::site();
        let documento = documento_com(titulo, descricao, slug, corpo);
        let _ = validar(&documento, &Contexto::vazio());

        if let Ok(cabecalho) = montar_cabecalho(&documento, &site) {
            let html = cabecalho.para_html();
            prop_assert_eq!(html.matches("<title>").count(), 1);
            prop_assert_eq!(html.matches("<script").count(), cabecalho.json_ld.len());
            prop_assert_eq!(html.matches("</script>").count(), cabecalho.json_ld.len());
        }

        let documentos = [documento];
        let itens = ItensDoSite { documentos: &documentos, categorias: &[], destaques: &[] };
        let _ = gerar_sitemaps(&site, &itens);
        let _ = gerar_llms(&site, &itens);
    }

    #[test]
    fn caminho_qualquer_nao_quebra_canonica_nem_sitemap(caminho in texto()) {
        let site = demonstracao::site();
        prop_assert!(canonica(&site, &caminho).starts_with(&site.origem));

        let mut documento = demonstracao::post_de_exemplo();
        documento.caminho = caminho;
        let documentos = [documento];
        let itens = ItensDoSite { documentos: &documentos, categorias: &[], destaques: &[] };
        for arquivo in gerar_sitemaps(&site, &itens) {
            // Fora das tags do próprio gerador, nenhum `<` sobra sem escape.
            for linha in arquivo.conteudo.lines().filter(|l| l.contains("<loc>")) {
                let valor = linha.trim().trim_start_matches("<loc>").trim_end_matches("</loc>");
                prop_assert!(!valor.contains('<') && !valor.contains('>'), "{}", linha);
            }
        }
    }

    #[test]
    fn robots_nao_ganha_regra_pelo_texto(caminho in texto(), robo in texto()) {
        let opcoes = OpcoesRobots { bloquear: vec![caminho], robos_ia_bloqueados: vec![robo.clone()] };
        let saida = gerar_robots(&demonstracao::site(), &opcoes);
        let esperado = if robo.split_whitespace().next().is_some() { 2 } else { 1 };
        prop_assert_eq!(saida.lines().filter(|l| l.starts_with("User-agent:")).count(), esperado);
        prop_assert_eq!(saida.lines().filter(|l| l.starts_with("Allow:")).count(), 1);
        prop_assert_eq!(saida.lines().filter(|l| l.starts_with("Sitemap:")).count(), 1);
    }

    #[test]
    fn upload_qualquer_nao_entra_em_panico(nome in texto(), bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        let _ = validar_upload(&nome, &bytes);
    }
}
