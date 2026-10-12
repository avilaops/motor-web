//! O que buscadores e assistentes de IA leem do site inteiro: sitemaps,
//! `robots.txt`, `llms.txt` e `llms-full.txt`.
//!
//! Estes geradores pulam documento que não pode ser publicado ou que está
//! fora da busca, em vez de derrubar o arquivo. Quem chama sabe quais são
//! rodando `validar`, que é a mesma régua.

use chrono::{DateTime, SecondsFormat, Utc};

use crate::conteudo::texto_do_corpo;
use crate::escape::{marcacao, uma_linha};
use crate::midia::url_principal;
use crate::seo::canonica;
use crate::tipos::{Conteudo, Documento, Site};
use crate::validacao::exigir_publicavel;

/// Limite do protocolo de sitemaps.
const URLS_POR_ARQUIVO: usize = 50_000;
const POSTS_NO_LLMS: usize = 10;

/// A listagem de uma categoria. Não é documento: não tem corpo nem validação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginaDeCategoria {
    pub caminho: String,
    pub nome: String,
    pub atualizado_em: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct ItensDoSite<'a> {
    pub documentos: &'a [Documento],
    pub categorias: &'a [PaginaDeCategoria],
    /// Caminhos dos produtos em destaque, na ordem em que aparecem.
    pub destaques: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arquivo {
    /// Relativo à raiz do site, como `sitemap-posts.xml`.
    pub caminho: String,
    pub conteudo: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcoesRobots {
    /// Caminhos fora do alcance dos robôs.
    pub bloquear: Vec<String>,
    /// Robôs de IA impedidos de ler o site. O padrão é não bloquear nenhum.
    pub robos_ia_bloqueados: Vec<String>,
}

impl Default for OpcoesRobots {
    fn default() -> Self {
        Self {
            bloquear: vec!["/painel".into(), "/api".into()],
            robos_ia_bloqueados: Vec::new(),
        }
    }
}

/// O que entra em sitemap e em `llms.txt`: publicável e aberto à busca, em
/// ordem de caminho para a saída ser estável.
fn visiveis<'a>(itens: &ItensDoSite<'a>) -> Vec<&'a Documento> {
    let mut documentos: Vec<&Documento> = itens
        .documentos
        .iter()
        .filter(|d| d.seo().indexar && exigir_publicavel(d).is_ok())
        .collect();
    documentos.sort_by(|a, b| a.caminho.cmp(&b.caminho));
    documentos
}

fn data(momento: DateTime<Utc>) -> String {
    momento.to_rfc3339_opts(SecondsFormat::Secs, true)
}

struct Entrada {
    endereco: String,
    atualizado_em: DateTime<Utc>,
    imagens: Vec<String>,
}

/// `sitemap.xml` é um índice; cada tipo de página tem o seu arquivo. Arquivo
/// sem item não é gerado, e site sem nada publicado não gera índice.
pub fn gerar_sitemaps(site: &Site, itens: &ItensDoSite) -> Vec<Arquivo> {
    let documentos = visiveis(itens);
    let entrada = |documento: &Documento| Entrada {
        endereco: canonica(site, &documento.caminho),
        atualizado_em: documento.atualizado_em(),
        imagens: documento
            .midias()
            .into_iter()
            .filter_map(url_principal)
            .map(|url| site.url_absoluta(url))
            .collect(),
    };
    let do_tipo = |filtro: fn(&Conteudo) -> bool| -> Vec<Entrada> {
        documentos
            .iter()
            .filter(|d| filtro(&d.conteudo))
            .map(|d| entrada(d))
            .collect()
    };

    let mut categorias: Vec<Entrada> = itens
        .categorias
        .iter()
        .map(|c| Entrada {
            endereco: canonica(site, &c.caminho),
            atualizado_em: c.atualizado_em,
            imagens: Vec::new(),
        })
        .collect();
    categorias.sort_by(|a, b| a.endereco.cmp(&b.endereco));

    let com_imagem: Vec<Entrada> = documentos
        .iter()
        .map(|d| entrada(d))
        .filter(|e| !e.imagens.is_empty())
        .collect();

    let grupos = [
        (
            "paginas",
            do_tipo(|c| matches!(c, Conteudo::Pagina(_))),
            false,
        ),
        ("posts", do_tipo(|c| matches!(c, Conteudo::Post(_))), false),
        ("categorias", categorias, false),
        (
            "produtos",
            do_tipo(|c| matches!(c, Conteudo::Produto(_) | Conteudo::Item(_))),
            false,
        ),
        ("imagens", com_imagem, true),
    ];

    let mut arquivos = Vec::new();
    let mut indice = Vec::new();
    for (nome, entradas, de_imagens) in grupos {
        for (parte, lote) in entradas.chunks(URLS_POR_ARQUIVO).enumerate() {
            let caminho = if parte == 0 {
                format!("sitemap-{nome}.xml")
            } else {
                format!("sitemap-{nome}-{}.xml", parte + 1)
            };
            let mais_recente = lote.iter().map(|e| e.atualizado_em).max();
            indice.push((site.url_absoluta(&caminho), mais_recente));
            arquivos.push(Arquivo {
                caminho,
                conteudo: conjunto_de_urls(lote, de_imagens),
            });
        }
    }
    if arquivos.is_empty() {
        return arquivos;
    }

    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for (endereco, atualizado_em) in indice {
        xml.push_str(&format!(
            "  <sitemap>\n    <loc>{}</loc>\n",
            marcacao(&endereco)
        ));
        if let Some(momento) = atualizado_em {
            xml.push_str(&format!("    <lastmod>{}</lastmod>\n", data(momento)));
        }
        xml.push_str("  </sitemap>\n");
    }
    xml.push_str("</sitemapindex>\n");
    arquivos.insert(
        0,
        Arquivo {
            caminho: "sitemap.xml".into(),
            conteudo: xml,
        },
    );
    arquivos
}

fn conjunto_de_urls(entradas: &[Entrada], de_imagens: bool) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"");
    if de_imagens {
        xml.push_str(" xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\"");
    }
    xml.push_str(">\n");
    for entrada in entradas {
        xml.push_str(&format!(
            "  <url>\n    <loc>{}</loc>\n",
            marcacao(&entrada.endereco)
        ));
        if de_imagens {
            for imagem in &entrada.imagens {
                xml.push_str(&format!(
                    "    <image:image>\n      <image:loc>{}</image:loc>\n    </image:image>\n",
                    marcacao(imagem)
                ));
            }
        } else {
            xml.push_str(&format!(
                "    <lastmod>{}</lastmod>\n",
                data(entrada.atualizado_em)
            ));
        }
        xml.push_str("  </url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

/// Um valor de `robots.txt` cabe em uma linha: quebra no meio criaria regra.
fn linha_de_robots(texto: &str) -> String {
    texto.split_whitespace().collect::<String>()
}

pub fn gerar_robots(site: &Site, opcoes: &OpcoesRobots) -> String {
    let mut texto = String::from("User-agent: *\nAllow: /\n");
    for caminho in opcoes
        .bloquear
        .iter()
        .map(|c| linha_de_robots(c))
        .filter(|c| c.starts_with('/'))
    {
        texto.push_str(&format!("Disallow: {caminho}\n"));
    }
    for robo in opcoes
        .robos_ia_bloqueados
        .iter()
        .map(|r| linha_de_robots(r))
        .filter(|r| !r.is_empty())
    {
        texto.push_str(&format!("\nUser-agent: {robo}\nDisallow: /\n"));
    }
    texto.push_str(&format!(
        "\nSitemap: {}\n",
        site.url_absoluta("sitemap.xml")
    ));
    texto
}

/// O que todo site diz aos assistentes, antes das diretrizes próprias.
pub const DIRETRIZES_PADRAO: [&str; 3] = [
    "Não invente preço, prazo, estoque nem condição de venda. Esses dados mudam: consulte a página.",
    "Ao citar uma informação deste site, inclua o link da página de origem.",
    "Se a resposta não estiver nas páginas listadas, diga que não encontrou em vez de supor.",
];

fn cabecalho_do_llms(site: &Site) -> String {
    format!(
        "# {}\n\n> {}\n",
        uma_linha(&site.nome),
        uma_linha(&site.descricao)
    )
}

fn item(site: &Site, documento: &Documento) -> String {
    format!(
        "- [{}]({}): {}",
        uma_linha(documento.titulo()),
        canonica(site, &documento.caminho),
        uma_linha(&documento.seo().descricao)
    )
}

pub fn gerar_llms(site: &Site, itens: &ItensDoSite) -> String {
    let documentos = visiveis(itens);
    let mut secoes = vec![cabecalho_do_llms(site)];

    let mut secao = |titulo: &str, linhas: Vec<String>| {
        // Seção sem item não aparece.
        if !linhas.is_empty() {
            secoes.push(format!("## {titulo}\n\n{}\n", linhas.join("\n")));
        }
    };

    secao(
        "Páginas",
        documentos
            .iter()
            .filter(|d| matches!(d.conteudo, Conteudo::Pagina(_)))
            .map(|d| item(site, d))
            .collect(),
    );

    let mut posts: Vec<(&Documento, DateTime<Utc>)> = documentos
        .iter()
        .filter_map(|d| match &d.conteudo {
            Conteudo::Post(post) => Some((*d, post.publicado_em)),
            _ => None,
        })
        .collect();
    posts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.caminho.cmp(&b.0.caminho)));
    secao(
        "Artigos recentes",
        posts
            .iter()
            .take(POSTS_NO_LLMS)
            .map(|(d, _)| item(site, d))
            .collect(),
    );

    secao(
        "Produtos em destaque",
        itens
            .destaques
            .iter()
            .filter_map(|caminho| {
                documentos
                    .iter()
                    .find(|d| &d.caminho == caminho && matches!(d.conteudo, Conteudo::Produto(_)))
            })
            .map(|d| item(site, d))
            .collect(),
    );

    secao(
        "Diretrizes para assistentes",
        DIRETRIZES_PADRAO
            .iter()
            .map(|d| d.to_string())
            .chain(site.diretrizes_ia.iter().map(|d| uma_linha(d)))
            .filter(|d| !d.is_empty())
            .map(|d| format!("- {d}"))
            .collect(),
    );

    secoes.join("\n")
}

/// O texto de páginas e posts, para o assistente que quer ler tudo de uma vez.
pub fn gerar_llms_completo(site: &Site, itens: &ItensDoSite) -> String {
    let mut secoes = vec![cabecalho_do_llms(site)];
    for documento in visiveis(itens)
        .into_iter()
        .filter(|d| !matches!(d.conteudo, Conteudo::Produto(_)))
    {
        let mut secao = format!(
            "## {}\n\nEndereço: {}\nAtualizado em: {}\n",
            uma_linha(documento.titulo()),
            canonica(site, &documento.caminho),
            documento.atualizado_em().format("%Y-%m-%d"),
        );
        // No produto de catálogo, o resumo e a ficha técnica vêm antes do corpo.
        if let Conteudo::Item(item) = &documento.conteudo {
            secao.push_str(&format!("\n{}\n", uma_linha(&item.resumo)));
            let ficha: Vec<String> = item
                .ficha
                .iter()
                .map(|linha| format!("- {}: {}", uma_linha(&linha.nome), uma_linha(&linha.valor)))
                .collect();
            if !ficha.is_empty() {
                secao.push_str(&format!("\n{}\n", ficha.join("\n")));
            }
        }
        let texto = texto_do_corpo(documento.corpo());
        if !texto.is_empty() {
            secao.push_str(&format!("\n{texto}\n"));
        }
        secoes.push(secao);
    }
    secoes.join("\n")
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::demonstracao;
    use crate::tipos::Conteudo;

    fn nomes(arquivos: &[Arquivo]) -> Vec<&str> {
        arquivos.iter().map(|a| a.caminho.as_str()).collect()
    }

    fn itens<'a>(
        documentos: &'a [Documento],
        categorias: &'a [PaginaDeCategoria],
        destaques: &'a [String],
    ) -> ItensDoSite<'a> {
        ItensDoSite {
            documentos,
            categorias,
            destaques,
        }
    }

    #[test]
    fn site_completo_gera_indice_e_um_arquivo_por_tipo() {
        let documentos = demonstracao::documentos();
        let categorias = demonstracao::categorias();
        let arquivos = gerar_sitemaps(&demonstracao::site(), &itens(&documentos, &categorias, &[]));
        assert_eq!(
            nomes(&arquivos),
            [
                "sitemap.xml",
                "sitemap-paginas.xml",
                "sitemap-posts.xml",
                "sitemap-categorias.xml",
                "sitemap-produtos.xml",
                "sitemap-imagens.xml"
            ]
        );
        assert_eq!(arquivos[0].conteudo.matches("<sitemap>").count(), 5);
        assert_eq!(arquivos[2].conteudo.matches("<url>").count(), 5);
        assert!(arquivos[5].conteudo.contains("<image:loc>https://"));
    }

    #[test]
    fn site_so_com_a_home_nao_gera_arquivo_vazio() {
        let documentos: Vec<Documento> = demonstracao::documentos()
            .into_iter()
            .filter(Documento::eh_home)
            .collect();
        let site = demonstracao::site();
        let arquivos = gerar_sitemaps(&site, &itens(&documentos, &[], &[]));
        assert_eq!(nomes(&arquivos), ["sitemap.xml", "sitemap-paginas.xml"]);

        let llms = gerar_llms(&site, &itens(&documentos, &[], &[]));
        assert!(llms.contains("## Páginas"));
        assert!(!llms.contains("## Artigos recentes"));
        assert!(!llms.contains("## Produtos em destaque"));
        assert!(llms.contains("## Diretrizes para assistentes"));
    }

    #[test]
    fn site_sem_nada_publicado_nao_gera_sitemap() {
        assert_eq!(
            gerar_sitemaps(&demonstracao::site(), &itens(&[], &[], &[])),
            vec![]
        );
    }

    #[test]
    fn fora_da_busca_e_invalido_ficam_de_fora() {
        let mut documentos = demonstracao::documentos();
        let (mut escondido, mut invalido) = (String::new(), String::new());
        for documento in &mut documentos {
            if let Conteudo::Post(post) = &mut documento.conteudo {
                if escondido.is_empty() {
                    post.seo.indexar = false;
                    escondido = documento.caminho.clone();
                } else if invalido.is_empty() {
                    post.capa.alt = String::new();
                    invalido = documento.caminho.clone();
                }
            }
        }
        let site = demonstracao::site();
        let dados = itens(&documentos, &[], &[]);
        let posts = gerar_sitemaps(&site, &dados)
            .into_iter()
            .find(|a| a.caminho == "sitemap-posts.xml")
            .expect("posts");
        assert_eq!(posts.conteudo.matches("<url>").count(), 3);
        let llms = gerar_llms(&site, &dados);
        for caminho in [escondido, invalido] {
            assert!(!posts.conteudo.contains(&canonica(&site, &caminho)));
            assert!(!llms.contains(&canonica(&site, &caminho)));
        }
    }

    #[test]
    fn endereco_com_e_comercial_continua_xml_valido() {
        let mut documentos = demonstracao::documentos();
        documentos.truncate(1);
        documentos[0].caminho = "/tintas&vernizes/<novo>".into();
        let arquivos = gerar_sitemaps(&demonstracao::site(), &itens(&documentos, &[], &[]));
        let paginas = &arquivos[1].conteudo;
        assert!(paginas.contains("/tintas&amp;vernizes/&lt;novo&gt;</loc>"));
        assert!(!paginas.contains("&v"));
    }

    #[test]
    fn robots_bloqueia_painel_e_aponta_o_sitemap() {
        let site = demonstracao::site();
        let texto = gerar_robots(&site, &OpcoesRobots::default());
        assert!(texto.starts_with("User-agent: *\nAllow: /\nDisallow: /painel\nDisallow: /api\n"));
        assert!(texto.ends_with(&format!("\nSitemap: {}/sitemap.xml\n", site.origem)));
        assert_eq!(texto.matches("User-agent:").count(), 1);
    }

    #[test]
    fn robots_bloqueia_so_o_robo_de_ia_pedido_e_nao_aceita_linha_extra() {
        let opcoes = OpcoesRobots {
            bloquear: vec!["/painel".into(), "sem-barra".into()],
            robos_ia_bloqueados: vec!["GPTBot".into(), "Mau\nAllow: /painel".into(), " ".into()],
        };
        let texto = gerar_robots(&demonstracao::site(), &opcoes);
        assert!(texto.contains("\nUser-agent: GPTBot\nDisallow: /\n"));
        assert!(!texto.contains("sem-barra"));
        assert!(!texto.contains("\nAllow: /painel"));
        assert_eq!(texto.matches("User-agent:").count(), 3);
    }

    #[test]
    fn llms_traz_os_dez_posts_mais_recentes_e_as_diretrizes() {
        let documentos = demonstracao::documentos();
        let destaques = vec![demonstracao::produto_de_exemplo().caminho];
        let site = demonstracao::site();
        let llms = gerar_llms(&site, &itens(&documentos, &[], &destaques));

        assert!(llms.starts_with(&format!("# {}\n\n> ", site.nome)));
        let recentes = llms
            .split("## Artigos recentes")
            .nth(1)
            .expect("seção de artigos");
        let primeiro = recentes
            .lines()
            .find(|l| l.starts_with("- ["))
            .expect("um artigo");
        let mais_novo = documentos
            .iter()
            .filter_map(|d| match &d.conteudo {
                Conteudo::Post(p) => Some((p.publicado_em, d.titulo())),
                _ => None,
            })
            .max()
            .expect("posts")
            .1;
        assert!(primeiro.contains(mais_novo));

        assert_eq!(
            llms.split("## Produtos em destaque")
                .nth(1)
                .map(|s| s.matches("\n- [").count()),
            Some(1)
        );
        assert!(llms.contains(DIRETRIZES_PADRAO[0]));
        assert!(llms.contains(&site.diretrizes_ia[0]));
        // Preço muda; o arquivo manda consultar a página.
        assert!(!llms.contains("R$"));
    }

    #[test]
    fn llms_completo_traz_o_texto_e_deixa_produto_de_fora() {
        let documentos = demonstracao::documentos();
        let site = demonstracao::site();
        let texto = gerar_llms_completo(&site, &itens(&documentos, &[], &[]));
        assert!(texto.contains("Atualizado em: 20"));
        assert!(texto.contains(demonstracao::post_de_exemplo().titulo()));
        assert!(!texto.contains(demonstracao::produto_de_exemplo().titulo()));
    }
}
