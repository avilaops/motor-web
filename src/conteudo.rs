//! O corpo do documento: de blocos tipados para HTML e para Markdown.

use crate::escape::{marcacao, uma_linha};
use crate::midia::{Papel, renderizar_midia, url_principal};
use crate::secoes;
use crate::tipos::{Bloco, Midia, Modelo3d, Trecho};
use crate::url::{incorporacao_de_video, link_valido};

/// Largura da coluna de texto, para o navegador escolher a variante certa.
const SIZES_DO_CORPO: &str = "(min-width: 800px) 768px, 100vw";

/// HTML semântico do corpo. Todo texto e todo atributo são escapados, e o
/// corpo nunca gera `<h1>`.
pub fn renderizar_corpo(blocos: &[Bloco]) -> String {
    blocos
        .iter()
        .enumerate()
        .map(|(indice, bloco)| renderizar_bloco(bloco, indice))
        .filter(|html| !html.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `indice` é a posição do bloco no corpo: o formulário a leva no envio, para
/// o site saber qual formulário da página foi preenchido.
fn renderizar_bloco(bloco: &Bloco, indice: usize) -> String {
    match bloco {
        Bloco::Paragrafo { trechos } => {
            format!(
                "<p>{}</p>",
                trechos.iter().map(renderizar_trecho).collect::<String>()
            )
        }
        Bloco::Titulo { nivel, texto } => {
            let nivel = (*nivel).clamp(2, 4);
            format!("<h{nivel}>{}</h{nivel}>", marcacao(texto))
        }
        Bloco::Imagem { midia } => renderizar_midia(midia, Papel::Conteudo, SIZES_DO_CORPO),
        Bloco::Lista { ordenada, itens } => {
            let tag = if *ordenada { "ol" } else { "ul" };
            let itens: String = itens
                .iter()
                .map(|item| format!("<li>{}</li>", marcacao(item)))
                .collect();
            format!("<{tag}>{itens}</{tag}>")
        }
        Bloco::Citacao { texto, fonte } => {
            let fonte = fonte
                .as_deref()
                .filter(|f| !f.trim().is_empty())
                .map(|f| format!("<footer>{}</footer>", marcacao(f)))
                .unwrap_or_default();
            format!("<blockquote><p>{}</p>{fonte}</blockquote>", marcacao(texto))
        }
        Bloco::Tabela { cabecalho, linhas } => {
            let cabecalho: String = cabecalho
                .iter()
                .map(|c| format!(r#"<th scope="col">{}</th>"#, marcacao(c)))
                .collect();
            let linhas: String = linhas
                .iter()
                .map(|linha| {
                    let celulas: String = linha
                        .iter()
                        .map(|c| format!("<td>{}</td>", marcacao(c)))
                        .collect();
                    format!("<tr>{celulas}</tr>")
                })
                .collect();
            format!("<table><thead><tr>{cabecalho}</tr></thead><tbody>{linhas}</tbody></table>")
        }
        Bloco::Perguntas { itens } => {
            // Lista de definição: não disputa nível de título com o resto do corpo.
            let itens: String = itens
                .iter()
                .map(|item| {
                    format!(
                        "<dt>{}</dt><dd>{}</dd>",
                        marcacao(&item.pergunta),
                        marcacao(&item.resposta)
                    )
                })
                .collect();
            format!(r#"<dl class="perguntas">{itens}</dl>"#)
        }
        Bloco::Video { url, titulo } => match incorporacao_de_video(url) {
            Some(endereco) => format!(
                r#"<iframe src="{}" title="{}" width="560" height="315" loading="lazy" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>"#,
                marcacao(&endereco),
                marcacao(titulo),
            ),
            None => String::new(),
        },
        Bloco::Modelo3d {
            modelo,
            capa,
            legenda,
        } => renderizar_modelo_3d(modelo, capa.as_ref(), legenda.as_deref()),
        Bloco::Chamada { texto, rotulo, url } => {
            let link = if link_valido(url) {
                format!(r#"<a href="{}">{}</a>"#, marcacao(url), marcacao(rotulo))
            } else {
                String::new()
            };
            format!(
                r#"<aside class="chamada"><p>{}</p>{link}</aside>"#,
                marcacao(texto)
            )
        }
        Bloco::Cartoes { .. }
        | Bloco::Destaque { .. }
        | Bloco::Depoimentos { .. }
        | Bloco::Numeros { .. }
        | Bloco::Passos { .. }
        | Bloco::Planos { .. }
        | Bloco::Galeria { .. }
        | Bloco::Logos { .. }
        | Bloco::Faixa { .. }
        | Bloco::Formulario { .. }
        | Bloco::Contato { .. } => secoes::renderizar(bloco, indice),
    }
}

fn renderizar_trecho(trecho: &Trecho) -> String {
    let mut html = marcacao(&trecho.texto);
    if trecho.italico {
        html = format!("<em>{html}</em>");
    }
    if trecho.negrito {
        html = format!("<strong>{html}</strong>");
    }
    // Link inválido não vira `href`: o texto sai, o destino não.
    if let Some(link) = trecho.link.as_deref().filter(|l| link_valido(l)) {
        html = format!(r#"<a href="{}">{html}</a>"#, marcacao(link));
    }
    html
}

/// O elemento `<modelo-3d>` é o visualizador do CMS: recebe o GLB em `src` e
/// o USDZ em `ios-src`. O que vai dentro dele é o que aparece antes de o
/// visualizador carregar e para quem está sem script: a capa e, no iPhone, o
/// atalho de realidade aumentada, que não depende de script nenhum (o Safari
/// abre o USDZ de um link `rel="ar"` que tenha uma imagem dentro).
fn renderizar_modelo_3d(modelo: &Modelo3d, capa: Option<&Midia>, legenda: Option<&str>) -> String {
    let imagem = capa
        .map(|capa| renderizar_midia(capa, Papel::Conteudo, SIZES_DO_CORPO))
        .unwrap_or_default();
    let ios = modelo
        .usdz
        .as_deref()
        .map(|usdz| format!(r#" ios-src="{}""#, marcacao(usdz)))
        .unwrap_or_default();
    let dentro = match (&modelo.usdz, imagem.is_empty()) {
        (Some(usdz), false) => format!(r#"<a rel="ar" href="{}">{imagem}</a>"#, marcacao(usdz)),
        _ => imagem,
    };
    let legenda = legenda
        .map(str::trim)
        .filter(|legenda| !legenda.is_empty())
        .map(|legenda| format!("<figcaption>{}</figcaption>", marcacao(legenda)))
        .unwrap_or_default();
    format!(
        r#"<figure class="modelo-3d"><modelo-3d src="{}"{ios} alt="{}">{dentro}</modelo-3d>{legenda}</figure>"#,
        marcacao(&modelo.glb),
        marcacao(&modelo.descricao),
    )
}

/// O corpo em Markdown. Alimenta o `llms-full.txt`.
pub fn texto_do_corpo(blocos: &[Bloco]) -> String {
    blocos
        .iter()
        .map(texto_do_bloco)
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn texto_do_bloco(bloco: &Bloco) -> String {
    match bloco {
        Bloco::Paragrafo { trechos } => trechos.iter().map(texto_do_trecho).collect(),
        Bloco::Titulo { nivel, texto } => {
            format!(
                "{} {}",
                "#".repeat(usize::from((*nivel).clamp(2, 4))),
                uma_linha(texto)
            )
        }
        Bloco::Imagem { midia } => match url_principal(midia) {
            Some(url) => format!("![{}]({url})", uma_linha(&midia.alt)),
            None => String::new(),
        },
        Bloco::Lista { ordenada, itens } => itens
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if *ordenada {
                    format!("{}. {}", i + 1, uma_linha(item))
                } else {
                    format!("- {}", uma_linha(item))
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Bloco::Citacao { texto, fonte } => {
            let mut saida = format!("> {}", uma_linha(texto));
            if let Some(fonte) = fonte.as_deref().filter(|f| !f.trim().is_empty()) {
                saida.push_str(&format!("\n> — {}", uma_linha(fonte)));
            }
            saida
        }
        Bloco::Tabela { cabecalho, linhas } => {
            let linha = |celulas: &[String]| {
                format!(
                    "| {} |",
                    celulas
                        .iter()
                        .map(|c| uma_linha(c).replace('|', "\\|"))
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            };
            let mut saida = vec![
                linha(cabecalho),
                format!("|{}", " --- |".repeat(cabecalho.len())),
            ];
            saida.extend(linhas.iter().map(|l| linha(l)));
            saida.join("\n")
        }
        Bloco::Perguntas { itens } => itens
            .iter()
            .map(|item| {
                format!(
                    "**{}**\n\n{}",
                    uma_linha(&item.pergunta),
                    uma_linha(&item.resposta)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
        Bloco::Video { url, titulo } => format!("[{}]({url})", uma_linha(titulo)),
        Bloco::Modelo3d { modelo, .. } => {
            format!(
                "[Modelo 3D: {}]({})",
                uma_linha(&modelo.descricao),
                modelo.glb
            )
        }
        Bloco::Chamada { texto, rotulo, url } => {
            format!("{} [{}]({url})", uma_linha(texto), uma_linha(rotulo))
        }
        Bloco::Cartoes { .. }
        | Bloco::Destaque { .. }
        | Bloco::Depoimentos { .. }
        | Bloco::Numeros { .. }
        | Bloco::Passos { .. }
        | Bloco::Planos { .. }
        | Bloco::Galeria { .. }
        | Bloco::Logos { .. }
        | Bloco::Faixa { .. }
        | Bloco::Formulario { .. }
        | Bloco::Contato { .. } => secoes::em_markdown(bloco),
    }
}

fn texto_do_trecho(trecho: &Trecho) -> String {
    let mut texto = trecho.texto.clone();
    if trecho.italico {
        texto = format!("_{texto}_");
    }
    if trecho.negrito {
        texto = format!("**{texto}**");
    }
    match trecho.link.as_deref().filter(|l| link_valido(l)) {
        Some(link) => format!("[{texto}]({link})"),
        None => texto,
    }
}

/// Palavras do texto corrido do corpo.
pub fn contar_palavras(blocos: &[Bloco]) -> usize {
    blocos
        .iter()
        .map(|bloco| match bloco {
            Bloco::Paragrafo { trechos } => trechos.iter().map(|t| palavras(&t.texto)).sum(),
            Bloco::Titulo { texto, .. }
            | Bloco::Citacao { texto, .. }
            | Bloco::Chamada { texto, .. } => palavras(texto),
            Bloco::Lista { itens, .. } => itens.iter().map(|i| palavras(i)).sum(),
            Bloco::Tabela { linhas, .. } => linhas.iter().flatten().map(|c| palavras(c)).sum(),
            Bloco::Perguntas { itens } => itens
                .iter()
                .map(|i| palavras(&i.pergunta) + palavras(&i.resposta))
                .sum(),
            Bloco::Imagem { .. } | Bloco::Video { .. } | Bloco::Modelo3d { .. } => 0,
            Bloco::Cartoes { .. }
            | Bloco::Destaque { .. }
            | Bloco::Depoimentos { .. }
            | Bloco::Numeros { .. }
            | Bloco::Passos { .. }
            | Bloco::Planos { .. }
            | Bloco::Galeria { .. }
            | Bloco::Logos { .. }
            | Bloco::Faixa { .. }
            | Bloco::Formulario { .. }
            | Bloco::Contato { .. } => palavras(&secoes::em_markdown(bloco)),
        })
        .sum()
}

fn palavras(texto: &str) -> usize {
    texto.split_whitespace().count()
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::tipos::Pergunta;

    fn trecho(texto: &str) -> Trecho {
        Trecho {
            texto: texto.into(),
            ..Trecho::default()
        }
    }

    #[test]
    fn paragrafo_com_marcas_e_link() {
        let blocos = [Bloco::Paragrafo {
            trechos: vec![
                trecho("Veja "),
                Trecho {
                    texto: "o guia".into(),
                    negrito: true,
                    link: Some("/guia".into()),
                    ..Trecho::default()
                },
                trecho("."),
            ],
        }];
        assert_eq!(
            renderizar_corpo(&blocos),
            r#"<p>Veja <a href="/guia"><strong>o guia</strong></a>.</p>"#
        );
        assert_eq!(texto_do_corpo(&blocos), "Veja [**o guia**](/guia).");
    }

    #[test]
    fn texto_de_quem_edita_nunca_vira_marcacao() {
        let blocos = [
            Bloco::Paragrafo {
                trechos: vec![trecho("<script>alert(1)</script>")],
            },
            Bloco::Titulo {
                nivel: 2,
                texto: "A & B <b>".into(),
            },
        ];
        let html = renderizar_corpo(&blocos);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("<h2>A &amp; B &lt;b&gt;</h2>"));
    }

    #[test]
    fn link_perigoso_perde_o_destino_e_mantem_o_texto() {
        let blocos = [Bloco::Paragrafo {
            trechos: vec![Trecho {
                texto: "clique".into(),
                link: Some("javascript:alert(1)".into()),
                ..Trecho::default()
            }],
        }];
        assert_eq!(renderizar_corpo(&blocos), "<p>clique</p>");
    }

    #[test]
    fn corpo_nunca_gera_h1() {
        let blocos = [
            Bloco::Titulo {
                nivel: 1,
                texto: "Tentativa".into(),
            },
            Bloco::Titulo {
                nivel: 9,
                texto: "Outra".into(),
            },
        ];
        let html = renderizar_corpo(&blocos);
        assert!(!html.contains("<h1"));
        assert_eq!(html, "<h2>Tentativa</h2>\n<h4>Outra</h4>");
    }

    #[test]
    fn video_vira_iframe_sem_cookie_e_origem_estranha_some() {
        let blocos = [
            Bloco::Video {
                url: "https://youtu.be/abc123".into(),
                titulo: "Demonstração".into(),
            },
            Bloco::Video {
                url: "https://exemplo.com/v.mp4".into(),
                titulo: "Outro".into(),
            },
        ];
        let html = renderizar_corpo(&blocos);
        assert!(html.contains(r#"src="https://www.youtube-nocookie.com/embed/abc123""#));
        assert!(html.contains(r#"loading="lazy""#));
        assert_eq!(html.matches("<iframe").count(), 1);
    }

    #[test]
    fn lista_tabela_e_perguntas() {
        let blocos = [
            Bloco::Lista {
                ordenada: true,
                itens: vec!["um".into(), "dois".into()],
            },
            Bloco::Tabela {
                cabecalho: vec!["Peça".into(), "Prazo".into()],
                linhas: vec![vec!["Mesa".into(), "30 dias".into()]],
            },
            Bloco::Perguntas {
                itens: vec![Pergunta {
                    pergunta: "Entrega?".into(),
                    resposta: "Sim.".into(),
                }],
            },
        ];
        let html = renderizar_corpo(&blocos);
        assert!(html.contains("<ol><li>um</li><li>dois</li></ol>"));
        assert!(html.contains(r#"<th scope="col">Peça</th>"#));
        assert!(html.contains("<dt>Entrega?</dt><dd>Sim.</dd>"));

        let texto = texto_do_corpo(&blocos);
        assert!(texto.contains("1. um\n2. dois"));
        assert!(texto.contains("| Peça | Prazo |\n| --- | --- |\n| Mesa | 30 dias |"));
        assert!(texto.contains("**Entrega?**\n\nSim."));
    }

    #[test]
    fn conta_palavras_do_texto() {
        let blocos = [
            Bloco::Paragrafo {
                trechos: vec![trecho("uma duas "), trecho("três")],
            },
            Bloco::Lista {
                ordenada: false,
                itens: vec!["quatro cinco".into()],
            },
        ];
        assert_eq!(contar_palavras(&blocos), 5);
    }

    #[test]
    fn modelo_3d_sai_com_o_visualizador_e_o_atalho_do_iphone() {
        use crate::tipos::Modelo3d;
        let modelo = Modelo3d {
            id: "m1".into(),
            glb: "/midia/modelos/m1.glb".into(),
            usdz: Some("/midia/modelos/m1.usdz".into()),
            descricao: "Leg press \"45 graus\"".into(),
        };
        let html = renderizar_corpo(&[Bloco::Modelo3d {
            modelo: modelo.clone(),
            capa: None,
            legenda: Some("Gire com o dedo".into()),
        }]);
        assert!(html.contains(
            r#"<modelo-3d src="/midia/modelos/m1.glb" ios-src="/midia/modelos/m1.usdz""#
        ));
        assert!(html.contains("alt=\"Leg press &quot;45 graus&quot;\""));
        assert!(html.contains("<figcaption>Gire com o dedo</figcaption>"));
        // Sem capa não há o que pôr dentro do atalho de realidade aumentada.
        assert!(!html.contains("rel=\"ar\""));

        let sem_iphone = renderizar_corpo(&[Bloco::Modelo3d {
            modelo: Modelo3d {
                usdz: None,
                ..modelo
            },
            capa: None,
            legenda: None,
        }]);
        assert!(!sem_iphone.contains("ios-src"));
        assert!(!sem_iphone.contains("figcaption"));
    }
}
