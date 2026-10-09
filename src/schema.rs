//! Dados estruturados (JSON-LD) por tipo de documento.

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Map, Value, json};

use crate::conteudo::contar_palavras;
use crate::midia::url_principal;
use crate::seo::canonica;
use crate::tipos::{
    Bloco, Conteudo, Disponibilidade, Documento, Midia, Pagina, Post, Product, Site, TipoPost,
    Variacao,
};
use crate::validacao::{DocumentoInvalido, exigir_publicavel};

const CONTEXTO: &str = "https://schema.org";

pub fn montar_json_ld(documento: &Documento, site: &Site) -> Result<Vec<Value>, DocumentoInvalido> {
    exigir_publicavel(documento)?;
    let endereco = canonica(site, &documento.caminho);

    let mut itens = match &documento.conteudo {
        Conteudo::Pagina(_) if documento.eh_home() => vec![organizacao(site), sitio(site)],
        Conteudo::Pagina(pagina) => vec![pagina_web(pagina, site, &endereco)],
        Conteudo::Post(post) => vec![artigo(post, site, &endereco)],
        Conteudo::Produto(produto) => vec![produto_ld(produto, site, &endereco)],
    };
    if !documento.eh_home() {
        itens.extend(trilha(documento, site));
    }
    itens.extend(perguntas(documento.corpo()));
    Ok(itens)
}

fn id_da_organizacao(site: &Site) -> String {
    format!("{}/#organizacao", site.origem_limpa())
}

fn id_do_site(site: &Site) -> String {
    format!("{}/#site", site.origem_limpa())
}

fn data(momento: DateTime<Utc>) -> String {
    momento.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn imagem(midia: &Midia, site: &Site) -> Option<String> {
    url_principal(midia).map(|url| site.url_absoluta(url))
}

/// Insere só o que tem valor: campo opcional ausente some da saída.
fn inserir(objeto: &mut Map<String, Value>, chave: &str, valor: Option<impl Into<Value>>) {
    if let Some(valor) = valor {
        objeto.insert(chave.to_string(), valor.into());
    }
}

fn texto(valor: &Option<String>) -> Option<String> {
    valor
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(String::from)
}

fn organizacao(site: &Site) -> Value {
    let dados = &site.organizacao;
    let mut objeto = Map::new();
    objeto.insert("@context".into(), CONTEXTO.into());
    objeto.insert("@type".into(), "Organization".into());
    objeto.insert("@id".into(), id_da_organizacao(site).into());
    objeto.insert("name".into(), site.nome.clone().into());
    objeto.insert("url".into(), format!("{}/", site.origem_limpa()).into());
    inserir(&mut objeto, "logo", imagem(&site.logo, site));
    inserir(&mut objeto, "legalName", texto(&dados.razao_social));
    inserir(&mut objeto, "telephone", texto(&dados.telefone));
    inserir(&mut objeto, "email", texto(&dados.email));
    if let Some(endereco) = &dados.endereco {
        objeto.insert(
            "address".into(),
            json!({
                "@type": "PostalAddress",
                "streetAddress": endereco.logradouro,
                "addressLocality": endereco.cidade,
                "addressRegion": endereco.uf,
                "postalCode": endereco.cep,
                "addressCountry": "BR",
            }),
        );
    }
    if !dados.perfis.is_empty() {
        objeto.insert("sameAs".into(), dados.perfis.clone().into());
    }
    Value::Object(objeto)
}

fn sitio(site: &Site) -> Value {
    json!({
        "@context": CONTEXTO,
        "@type": "WebSite",
        "@id": id_do_site(site),
        "name": site.nome,
        "url": format!("{}/", site.origem_limpa()),
        "description": site.descricao,
        "inLanguage": site.idioma,
        "publisher": { "@id": id_da_organizacao(site) },
    })
}

fn pagina_web(pagina: &Pagina, site: &Site, endereco: &str) -> Value {
    let mut objeto = Map::new();
    objeto.insert("@context".into(), CONTEXTO.into());
    objeto.insert("@type".into(), "WebPage".into());
    objeto.insert("name".into(), pagina.titulo.clone().into());
    objeto.insert("url".into(), endereco.into());
    objeto.insert("description".into(), pagina.seo.descricao.clone().into());
    objeto.insert("inLanguage".into(), site.idioma.clone().into());
    objeto.insert("datePublished".into(), data(pagina.publicado_em).into());
    objeto.insert("dateModified".into(), data(pagina.atualizado_em).into());
    inserir(
        &mut objeto,
        "primaryImageOfPage",
        pagina.capa.as_ref().and_then(|capa| imagem(capa, site)),
    );
    objeto.insert("isPartOf".into(), json!({ "@id": id_do_site(site) }));
    Value::Object(objeto)
}

fn artigo(post: &Post, site: &Site, endereco: &str) -> Value {
    let tipo = match post.tipo {
        TipoPost::Artigo => "Article",
        TipoPost::GuiaTecnico => "TechArticle",
    };

    let mut autor = Map::new();
    autor.insert("@type".into(), "Person".into());
    autor.insert("name".into(), post.autor.nome.clone().into());
    autor.insert("jobTitle".into(), post.autor.cargo.clone().into());
    autor.insert("description".into(), post.autor.bio.clone().into());
    inserir(&mut autor, "image", imagem(&post.autor.foto, site));
    if !post.autor.perfis.is_empty() {
        autor.insert("sameAs".into(), post.autor.perfis.clone().into());
    }

    let mut objeto = Map::new();
    objeto.insert("@context".into(), CONTEXTO.into());
    objeto.insert("@type".into(), tipo.into());
    objeto.insert("headline".into(), post.titulo.clone().into());
    objeto.insert("description".into(), post.resumo.clone().into());
    inserir(
        &mut objeto,
        "image",
        imagem(&post.capa, site).map(|url| vec![url]),
    );
    objeto.insert("datePublished".into(), data(post.publicado_em).into());
    objeto.insert("dateModified".into(), data(post.atualizado_em).into());
    objeto.insert("author".into(), Value::Object(autor));
    objeto.insert(
        "publisher".into(),
        json!({ "@id": id_da_organizacao(site) }),
    );
    objeto.insert("mainEntityOfPage".into(), endereco.into());
    objeto.insert("articleSection".into(), post.categoria.nome.clone().into());
    if !post.tags.is_empty() {
        objeto.insert("keywords".into(), post.tags.join(", ").into());
    }
    objeto.insert("wordCount".into(), contar_palavras(&post.corpo).into());
    objeto.insert("inLanguage".into(), site.idioma.clone().into());
    Value::Object(objeto)
}

fn preco(centavos: i64) -> String {
    format!("{}.{:02}", centavos / 100, centavos % 100)
}

fn disponibilidade(valor: Disponibilidade) -> &'static str {
    match valor {
        Disponibilidade::EmEstoque => "https://schema.org/InStock",
        Disponibilidade::SemEstoque => "https://schema.org/OutOfStock",
        Disponibilidade::SobEncomenda => "https://schema.org/BackOrder",
    }
}

fn oferta(variacao: &Variacao, produto: &Product, site: &Site, endereco: &str) -> Value {
    let mut objeto = Map::new();
    objeto.insert("@type".into(), "Offer".into());
    objeto.insert("url".into(), endereco.into());
    objeto.insert("name".into(), variacao.nome.clone().into());
    objeto.insert("sku".into(), variacao.sku.clone().into());
    inserir(&mut objeto, "gtin", texto(&variacao.gtin));
    objeto.insert("priceCurrency".into(), "BRL".into());
    objeto.insert("price".into(), preco(variacao.preco_centavos).into());
    objeto.insert(
        "availability".into(),
        disponibilidade(variacao.disponibilidade).into(),
    );
    objeto.insert(
        "hasMerchantReturnPolicy".into(),
        json!({
            "@type": "MerchantReturnPolicy",
            "applicableCountry": "BR",
            "merchantReturnLink": site.url_absoluta(&produto.politica_devolucao_url),
        }),
    );
    Value::Object(objeto)
}

fn produto_ld(produto: &Product, site: &Site, endereco: &str) -> Value {
    let imagens: Vec<String> = produto
        .imagens
        .iter()
        .filter_map(|m| imagem(m, site))
        .collect();
    let ofertas: Vec<Value> = produto
        .variacoes
        .iter()
        .map(|v| oferta(v, produto, site, endereco))
        .collect();

    let mut objeto = Map::new();
    objeto.insert("@context".into(), CONTEXTO.into());
    objeto.insert("@type".into(), "Product".into());
    objeto.insert("name".into(), produto.nome.clone().into());
    objeto.insert("description".into(), produto.descricao.clone().into());
    objeto.insert("url".into(), endereco.into());
    if !imagens.is_empty() {
        objeto.insert("image".into(), imagens.into());
    }
    inserir(
        &mut objeto,
        "sku",
        produto.variacoes.first().map(|v| v.sku.clone()),
    );
    if let Some(marca) = texto(&produto.marca) {
        objeto.insert("brand".into(), json!({ "@type": "Brand", "name": marca }));
    }
    // Só avaliação do próprio site, e só quando existe alguma.
    if let Some(avaliacoes) = produto.avaliacoes.as_ref().filter(|a| a.total > 0) {
        objeto.insert(
            "aggregateRating".into(),
            json!({
                "@type": "AggregateRating",
                "ratingValue": (f64::from(avaliacoes.media) * 10.0).round() / 10.0,
                "reviewCount": avaliacoes.total,
            }),
        );
    }

    let precos: Vec<i64> = produto.variacoes.iter().map(|v| v.preco_centavos).collect();
    match (ofertas.len(), precos.iter().min(), precos.iter().max()) {
        (1, _, _) => {
            objeto.insert(
                "offers".into(),
                ofertas.into_iter().next().unwrap_or(Value::Null),
            );
        }
        (quantidade, Some(menor), Some(maior)) => {
            objeto.insert(
                "offers".into(),
                json!({
                    "@type": "AggregateOffer",
                    "priceCurrency": "BRL",
                    "lowPrice": preco(*menor),
                    "highPrice": preco(*maior),
                    "offerCount": quantidade,
                    "offers": ofertas,
                }),
            );
        }
        _ => {}
    }
    Value::Object(objeto)
}

fn trilha(documento: &Documento, site: &Site) -> Option<Value> {
    let itens: Vec<Value> = documento
        .trilha()
        .iter()
        .enumerate()
        .map(|(i, item)| {
            json!({
                "@type": "ListItem",
                "position": i + 1,
                "name": item.nome,
                "item": site.url_absoluta(&item.url),
            })
        })
        .collect();
    (!itens.is_empty()).then(|| {
        json!({
            "@context": CONTEXTO,
            "@type": "BreadcrumbList",
            "itemListElement": itens,
        })
    })
}

fn perguntas(corpo: &[Bloco]) -> Option<Value> {
    let itens: Vec<Value> = corpo
        .iter()
        .filter_map(|bloco| match bloco {
            Bloco::Perguntas { itens } => Some(itens),
            _ => None,
        })
        .flatten()
        .map(|item| {
            json!({
                "@type": "Question",
                "name": item.pergunta,
                "acceptedAnswer": { "@type": "Answer", "text": item.resposta },
            })
        })
        .collect();
    (!itens.is_empty()).then(|| {
        json!({
            "@context": CONTEXTO,
            "@type": "FAQPage",
            "mainEntity": itens,
        })
    })
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::demonstracao;
    use crate::tipos::Avaliacoes;

    fn tipos(itens: &[Value]) -> Vec<&str> {
        itens
            .iter()
            .filter_map(|item| item["@type"].as_str())
            .collect()
    }

    fn produto(altera: impl FnOnce(&mut Product)) -> Value {
        let mut documento = demonstracao::produto_de_exemplo();
        if let Conteudo::Produto(produto) = &mut documento.conteudo {
            altera(produto);
        }
        montar_json_ld(&documento, &demonstracao::site())
            .expect("produto válido")
            .remove(0)
    }

    #[test]
    fn preco_sai_em_reais_com_duas_casas() {
        assert_eq!(preco(189_900), "1899.00");
        assert_eq!(preco(5), "0.05");
        assert_eq!(preco(1_050), "10.50");
    }

    #[test]
    fn uma_variacao_gera_offer() {
        let ld = produto(|p| p.variacoes.truncate(1));
        assert_eq!(ld["offers"]["@type"], "Offer");
        assert_eq!(ld["offers"]["priceCurrency"], "BRL");
        assert_eq!(ld["offers"]["availability"], "https://schema.org/InStock");
        assert!(
            ld["offers"]["hasMerchantReturnPolicy"]["merchantReturnLink"]
                .as_str()
                .is_some_and(|l| l.starts_with("https://"))
        );
    }

    #[test]
    fn varias_variacoes_geram_aggregate_offer() {
        let ld = produto(|p| {
            let base = p.variacoes[0].clone();
            p.variacoes = vec![
                Variacao {
                    preco_centavos: 30_000,
                    ..base.clone()
                },
                Variacao {
                    preco_centavos: 10_050,
                    ..base.clone()
                },
                Variacao {
                    preco_centavos: 20_000,
                    ..base
                },
            ];
        });
        assert_eq!(ld["offers"]["@type"], "AggregateOffer");
        assert_eq!(ld["offers"]["lowPrice"], "100.50");
        assert_eq!(ld["offers"]["highPrice"], "300.00");
        assert_eq!(ld["offers"]["offerCount"], 3);
        assert_eq!(ld["offers"]["offers"].as_array().map(Vec::len), Some(3));
    }

    #[test]
    fn nota_so_aparece_com_avaliacao_do_proprio_site() {
        let sem = produto(|p| p.avaliacoes = None);
        assert!(sem.get("aggregateRating").is_none());

        let zerada = produto(|p| {
            p.avaliacoes = Some(Avaliacoes {
                media: 5.0,
                total: 0,
            })
        });
        assert!(zerada.get("aggregateRating").is_none());

        let com = produto(|p| {
            p.avaliacoes = Some(Avaliacoes {
                media: 4.7,
                total: 12,
            })
        });
        assert_eq!(com["aggregateRating"]["ratingValue"], 4.7);
        assert_eq!(com["aggregateRating"]["reviewCount"], 12);
    }

    #[test]
    fn campo_opcional_ausente_some() {
        let ld = produto(|p| {
            p.marca = None;
            p.variacoes.truncate(1);
            p.variacoes[0].gtin = None;
        });
        assert!(ld.get("brand").is_none());
        assert!(ld["offers"].get("gtin").is_none());
    }

    #[test]
    fn tipos_por_documento() {
        let site = demonstracao::site();
        for documento in demonstracao::documentos() {
            let itens = montar_json_ld(&documento, &site).expect("fixture válida");
            let encontrados = tipos(&itens);
            match &documento.conteudo {
                Conteudo::Pagina(_) if documento.eh_home() => {
                    assert_eq!(encontrados, ["Organization", "WebSite"])
                }
                Conteudo::Pagina(_) => assert_eq!(encontrados[..2], ["WebPage", "BreadcrumbList"]),
                Conteudo::Post(post) => {
                    let esperado = if post.tipo == TipoPost::GuiaTecnico {
                        "TechArticle"
                    } else {
                        "Article"
                    };
                    assert_eq!(encontrados[..2], [esperado, "BreadcrumbList"]);
                }
                Conteudo::Produto(_) => assert_eq!(encontrados, ["Product", "BreadcrumbList"]),
            }
        }
    }

    #[test]
    fn documento_invalido_nao_gera_dados() {
        let mut documento = demonstracao::post_de_exemplo();
        if let Conteudo::Post(post) = &mut documento.conteudo {
            post.seo.titulo = String::new();
        }
        let erro = montar_json_ld(&documento, &demonstracao::site()).expect_err("título vazio");
        assert_eq!(erro.codigo, "seo.titulo.vazio");
    }
}
