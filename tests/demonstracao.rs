//! O site de demonstração atravessa o crate inteiro, e a saída é comparada
//! com as referências versionadas em `tests/snapshots`.

use motor_web::conteudo::renderizar_corpo;
use motor_web::demonstracao;
use motor_web::descoberta::{
    ItensDoSite, OpcoesRobots, gerar_llms, gerar_llms_completo, gerar_robots, gerar_sitemaps,
};
use motor_web::midia::{Papel, renderizar_midia};
use motor_web::schema::montar_json_ld;
use motor_web::seo::montar_cabecalho;
use motor_web::tipos::{Conteudo, Documento, Site};
use motor_web::validacao::{Contexto, validar};
use serde_json::Value;

fn documento(caminho: &str) -> Documento {
    demonstracao::documentos()
        .into_iter()
        .find(|d| d.caminho == caminho)
        .unwrap_or_else(|| panic!("sem {caminho}"))
}

/// A página como um template a montaria: título, capa e corpo.
fn pagina_montada(documento: &Documento) -> String {
    let capa = documento
        .capa()
        .map(|capa| renderizar_midia(capa, Papel::Principal, "100vw"))
        .unwrap_or_default();
    format!(
        "<h1>{}</h1>\n{capa}\n{}",
        documento.titulo(),
        renderizar_corpo(documento.corpo())
    )
}

#[test]
fn o_site_inteiro_passa_na_validacao_com_o_contexto_real() {
    let documentos = demonstracao::documentos();
    for (i, documento) in documentos.iter().enumerate() {
        let outros = || {
            documentos
                .iter()
                .enumerate()
                .filter(move |(j, _)| *j != i)
                .map(|(_, d)| d)
        };
        let titulos: Vec<String> = outros().map(|d| d.seo().titulo.clone()).collect();
        let descricoes: Vec<String> = outros().map(|d| d.seo().descricao.clone()).collect();
        let contexto = Contexto {
            titulos_em_uso: &titulos,
            descricoes_em_uso: &descricoes,
            ..Contexto::vazio()
        };
        assert_eq!(
            validar(documento, &contexto),
            vec![],
            "{}",
            documento.caminho
        );
    }
}

#[test]
fn o_site_tem_o_que_a_spec_pede() {
    let documentos = demonstracao::documentos();
    let conta =
        |filtro: fn(&Conteudo) -> bool| documentos.iter().filter(|d| filtro(&d.conteudo)).count();
    assert!(documentos[0].eh_home());
    assert_eq!(conta(|c| matches!(c, Conteudo::Pagina(_))), 4);
    assert_eq!(conta(|c| matches!(c, Conteudo::Post(_))), 5);
    assert_eq!(conta(|c| matches!(c, Conteudo::Produto(_))), 2);
}

#[test]
fn o_contrato_serde_vai_e_volta() {
    let site = demonstracao::site();
    let texto = serde_json::to_string(&site).expect("site em JSON");
    assert_eq!(
        serde_json::from_str::<Site>(&texto).expect("site de volta"),
        site
    );

    let documentos = demonstracao::documentos();
    let texto = serde_json::to_string(&documentos).expect("documentos em JSON");
    assert_eq!(
        serde_json::from_str::<Vec<Documento>>(&texto).expect("documentos de volta"),
        documentos
    );
}

#[test]
fn contrato_em_json() {
    insta::assert_snapshot!(
        "contrato_post",
        serde_json::to_string_pretty(&demonstracao::post_de_exemplo()).expect("JSON")
    );
}

#[test]
fn campo_desconhecido_e_recusado() {
    let mut valor = serde_json::to_value(demonstracao::post_de_exemplo()).expect("JSON");
    valor["conteudo"]["dados"]["html"] = Value::from("<script>alert(1)</script>");
    assert!(serde_json::from_value::<Documento>(valor).is_err());
}

#[test]
fn cabecalhos() {
    let site = demonstracao::site();
    for (nome, caminho) in [
        ("cabecalho_home", "/"),
        ("cabecalho_pagina", "/sobre"),
        ("cabecalho_post", "/blog/como-escolher-a-madeira-da-mesa"),
        ("cabecalho_guia", "/blog/guia-de-acabamentos-para-madeira"),
        (
            "cabecalho_perguntas",
            "/blog/perguntas-sobre-moveis-sob-medida",
        ),
        (
            "cabecalho_produto_com_variacoes",
            "/produtos/banco-de-madeira-macica",
        ),
        ("cabecalho_produto_simples", "/produtos/tabua-de-corte"),
    ] {
        let cabecalho = montar_cabecalho(&documento(caminho), &site).expect("fixture válida");
        insta::assert_snapshot!(nome, cabecalho.para_html());
    }
}

#[test]
fn corpos() {
    for (nome, caminho) in [
        ("corpo_post", "/blog/como-escolher-a-madeira-da-mesa"),
        ("corpo_guia", "/blog/guia-de-acabamentos-para-madeira"),
        ("corpo_perguntas", "/blog/perguntas-sobre-moveis-sob-medida"),
        ("corpo_servicos", "/servicos"),
    ] {
        insta::assert_snapshot!(nome, pagina_montada(&documento(caminho)));
    }
}

#[test]
fn descoberta() {
    let site = demonstracao::site();
    let documentos = demonstracao::documentos();
    let categorias = demonstracao::categorias();
    let destaques = vec!["/produtos/banco-de-madeira-macica".to_string()];
    let itens = ItensDoSite {
        documentos: &documentos,
        categorias: &categorias,
        destaques: &destaques,
    };

    let sitemaps: String = gerar_sitemaps(&site, &itens)
        .iter()
        .map(|arquivo| format!("=== {} ===\n{}", arquivo.caminho, arquivo.conteudo))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("sitemaps", sitemaps);
    insta::assert_snapshot!("robots", gerar_robots(&site, &OpcoesRobots::default()));
    insta::assert_snapshot!("llms", gerar_llms(&site, &itens));
    insta::assert_snapshot!("llms_completo", gerar_llms_completo(&site, &itens));
}

/// Campos que o Google exige para cada tipo aparecer como resultado rico.
#[test]
fn json_ld_tem_os_campos_obrigatorios_do_google() {
    let site = demonstracao::site();
    let presente = |item: &Value, campo: &str| {
        let valor = campo.split('.').fold(item, |valor, parte| &valor[parte]);
        assert!(
            !valor.is_null() && valor != &Value::from("") && valor != &Value::Array(vec![]),
            "{} sem {campo}: {item}",
            item["@type"]
        );
    };

    for documento in demonstracao::documentos() {
        for item in montar_json_ld(&documento, &site).expect("fixture válida") {
            let campos: &[&str] = match item["@type"].as_str().expect("@type") {
                "Organization" => &["name", "url", "logo"],
                "WebSite" => &["name", "url"],
                "WebPage" => &["name", "url"],
                "Article" | "TechArticle" => &[
                    "headline",
                    "image",
                    "datePublished",
                    "dateModified",
                    "author.name",
                    "publisher",
                ],
                "BreadcrumbList" => &["itemListElement"],
                "FAQPage" => &["mainEntity"],
                "Product" => &["name", "image", "offers"],
                outro => panic!("tipo inesperado: {outro}"),
            };
            for campo in campos {
                presente(&item, campo);
            }
            if item["@type"] == "Product" {
                let ofertas = &item["offers"];
                let lista = if ofertas["@type"] == "AggregateOffer" {
                    presente(ofertas, "lowPrice");
                    presente(ofertas, "priceCurrency");
                    ofertas["offers"].as_array().expect("ofertas").clone()
                } else {
                    vec![ofertas.clone()]
                };
                for oferta in lista {
                    for campo in ["price", "priceCurrency", "availability"] {
                        presente(&oferta, campo);
                    }
                }
            }
            if item["@type"] == "BreadcrumbList" {
                for elemento in item["itemListElement"].as_array().expect("itens") {
                    assert!(
                        elemento["item"]
                            .as_str()
                            .is_some_and(|url| url.starts_with("https://")),
                        "{elemento}"
                    );
                }
            }
        }
    }
}

#[test]
fn pagina_montada_tem_um_h1_uma_imagem_prioritaria_e_dimensoes_em_todas() {
    for documento in demonstracao::documentos() {
        let html = pagina_montada(&documento);
        assert_eq!(html.matches("<h1").count(), 1, "{}", documento.caminho);
        assert!(
            html.matches("fetchpriority=\"high\"").count() <= 1,
            "{}",
            documento.caminho
        );
        if documento.capa().is_some() {
            assert_eq!(
                html.matches("fetchpriority=\"high\"").count(),
                1,
                "{}",
                documento.caminho
            );
        }
        for imagem in html.split("<img ").skip(1) {
            let tag = imagem.split('>').next().unwrap_or("");
            assert!(
                tag.contains("width=\"") && tag.contains("height=\"") && tag.contains("alt=\""),
                "{tag}"
            );
        }
    }
}

#[test]
fn pagina_de_apresentacao_sai_com_abertura_e_todas_as_secoes() {
    let documento = demonstracao::apresentacao();
    assert!(documento.eh_larga());
    assert_eq!(validar(&documento, &Contexto::vazio()), vec![]);
    let Conteudo::Pagina(pagina) = &documento.conteudo else {
        panic!("a apresentação é uma página");
    };
    let html = format!(
        "{}\n{}",
        motor_web::secoes::renderizar_abertura(pagina).expect("abertura"),
        renderizar_corpo(&pagina.corpo)
    );
    assert_eq!(html.matches("<h1>").count(), 1);
    insta::assert_snapshot!("apresentacao_html", html);
    insta::assert_snapshot!(
        "apresentacao_contrato",
        serde_json::to_string_pretty(&documento.conteudo).expect("json")
    );
    // Página de texto corrido continua no leiaute de leitura.
    assert!(!documento_de_texto().eh_larga());
}

fn documento_de_texto() -> Documento {
    documento("/sobre")
}
