//! As seções de uma página de apresentação: do bloco tipado para HTML e para
//! Markdown, e a folha de estilo que vale em todo site.
//!
//! Seção é faixa de página inteira. O título dela é de nível 2 e o de cada
//! item, de nível 3. Todo texto e todo atributo são escapados, e nenhuma
//! seção gera `<h1>`: ele sai do título da página, na abertura.

use crate::escape::{marcacao, uma_linha};
use crate::midia::{Papel, renderizar_midia};
use crate::tipos::{Acao, Bloco, Campo, Midia, Pagina, TipoDeCampo};
use crate::url::link_valido;

const SIZES_DE_CARTAO: &str = "(min-width: 900px) 352px, (min-width: 600px) 50vw, 100vw";
const SIZES_DE_METADE: &str = "(min-width: 900px) 560px, 100vw";
const SIZES_DE_LOGO: &str = "160px";
const SIZES_DE_FOTO: &str = "48px";

/// O estilo das seções e da abertura. Usa as variáveis de cor do tema de quem
/// inclui (`--destaque`, `--suave`, `--linha`), com reserva para cada uma.
pub const ESTILO_DAS_SECOES: &str = "\
.secao{margin:3.5rem 0}\
.secao>h2{margin-top:0}\
.s-intro{font-size:1.125rem;color:var(--suave,#57534e);max-width:40rem}\
.s-grade{display:grid;grid-template-columns:repeat(auto-fit,minmax(16rem,1fr));gap:1.5rem;list-style:none;margin:1.5rem 0 0;padding:0}\
.s-cartao,.s-plano,.s-depoimento{border:1px solid var(--linha,#e7e5e4);border-radius:.75rem;padding:1.25rem;margin:0;height:100%}\
.s-grade h3{margin:.75rem 0 .25rem;font-size:1.125rem}\
.s-grade p{margin:.25rem 0}\
.s-cartao img{aspect-ratio:3/2;object-fit:cover;width:100%}\
.botao{display:inline-block;padding:.7rem 1.1rem;border-radius:.375rem;background:var(--destaque,#9a3412);color:#fff;text-decoration:none;font-weight:600;border:2px solid var(--destaque,#9a3412)}\
.botao.secundario{background:transparent;color:var(--destaque,#9a3412)}\
.s-acoes{display:flex;flex-wrap:wrap;gap:.75rem;margin:1.25rem 0 0}\
.s-destaque,.abertura.com-imagem{display:grid;gap:2rem;align-items:center}\
@media(min-width:56rem){.s-destaque,.abertura.com-imagem{grid-template-columns:1fr 1fr}.s-destaque.invertido .s-imagem{order:2}}\
.s-destaque h2{margin-top:0}\
.s-imagem figure{margin:0}\
.abertura{margin:1rem 0 3.5rem}\
.abertura h1{font-size:clamp(2rem,6vw,3.25rem)}\
.s-sobretitulo{margin:0;font-weight:600;color:var(--destaque,#9a3412);text-transform:uppercase;letter-spacing:.06em;font-size:.875rem}\
.s-depoimento blockquote{margin:0;padding:0;border:0}\
.s-depoimento figcaption{display:flex;align-items:center;gap:.75rem;margin-top:1rem;font-size:1rem}\
.s-depoimento figcaption img{width:48px;height:48px;border-radius:50%;object-fit:cover}\
.s-depoimento strong{display:block}\
.s-numeros{display:grid;grid-template-columns:repeat(auto-fit,minmax(10rem,1fr));gap:1.5rem;margin:1.5rem 0 0}\
.s-numeros div{display:flex;flex-direction:column-reverse}\
.s-numeros dd{margin:0;font-size:2.25rem;font-weight:700;line-height:1.1}\
.s-numeros dt{color:var(--suave,#57534e)}\
.s-passos{counter-reset:passo;list-style:none;padding:0;display:grid;grid-template-columns:repeat(auto-fit,minmax(14rem,1fr));gap:1.5rem;margin:1.5rem 0 0}\
.s-passos li{counter-increment:passo}\
.s-passos li::before{content:counter(passo);display:inline-grid;place-items:center;width:2.25rem;height:2.25rem;border-radius:50%;background:var(--destaque,#9a3412);color:#fff;font-weight:700}\
.s-passos h3{margin:.75rem 0 .25rem;font-size:1.125rem}\
.s-passos p{margin:0}\
.s-plano.recomendado{border:2px solid var(--destaque,#9a3412)}\
.s-preco strong{font-size:1.75rem}\
.s-plano ul{padding-left:1.1rem;margin:.75rem 0 1rem}\
.s-galeria,.s-logos{display:grid;gap:1rem;list-style:none;padding:0;margin:1.5rem 0 0}\
.s-galeria{grid-template-columns:repeat(auto-fit,minmax(14rem,1fr))}\
.s-galeria img{aspect-ratio:4/3;object-fit:cover;width:100%}\
.s-galeria figure,.s-logos figure{margin:0}\
.s-logos{grid-template-columns:repeat(auto-fit,minmax(7rem,1fr));align-items:center}\
.s-logos img{max-height:3.5rem;width:auto;margin:0 auto;border-radius:0}\
.s-faixa{padding:2.5rem 1.5rem;border-radius:.75rem;background:var(--destaque,#9a3412);color:#fff;text-align:center}\
.s-faixa h2{margin:0 0 .5rem}\
.s-faixa p{margin:0}\
.s-faixa .s-acoes{justify-content:center}\
.s-faixa .botao{background:#fff;color:var(--destaque,#9a3412);border-color:#fff}\
.s-faixa .botao.secundario{background:transparent;color:#fff}\
.s-canais{list-style:none;padding:0;margin:1.5rem 0 0;display:grid;gap:.75rem}\
.s-canais span{display:block;font-size:.9375rem;color:var(--suave,#57534e)}\
.s-formulario form{display:grid;gap:.5rem;max-width:34rem;margin-top:1.5rem}\
.s-formulario label{font-weight:600;margin-top:.5rem}\
.s-formulario input,.s-formulario textarea{font:inherit;padding:.65rem .75rem;border:1px solid var(--suave,#57534e);border-radius:.375rem;width:100%;background:#fff;color:inherit}\
.s-formulario input:focus-visible,.s-formulario textarea:focus-visible{outline:3px solid var(--destaque,#9a3412);outline-offset:1px}\
.s-formulario button{cursor:pointer;justify-self:start;margin-top:1rem;font:inherit;font-weight:600}\
.s-isca{position:absolute;left:-9999px;width:1px;height:1px;overflow:hidden}";

fn preenchido(texto: &Option<String>) -> Option<&str> {
    texto.as_deref().map(str::trim).filter(|t| !t.is_empty())
}

/// Um parágrafo por trecho separado por linha em branco.
fn paragrafos(texto: &str) -> String {
    texto
        .split("\n\n")
        .map(str::trim)
        .filter(|trecho| !trecho.is_empty())
        .map(|trecho| format!("<p>{}</p>", marcacao(trecho)))
        .collect()
}

/// Botão com destino inválido não sai: o texto sem `href` enganaria.
fn botao(acao: &Acao, classe: &str) -> String {
    if !link_valido(&acao.url) || acao.rotulo.trim().is_empty() {
        return String::new();
    }
    format!(
        r#"<a class="{classe}" href="{}">{}</a>"#,
        marcacao(&acao.url),
        marcacao(&acao.rotulo)
    )
}

/// A linha de botões: o primeiro é o principal.
fn botoes(acoes: &[Acao]) -> String {
    let botoes: String = acoes
        .iter()
        .enumerate()
        .map(|(i, acao)| botao(acao, if i == 0 { "botao" } else { "botao secundario" }))
        .collect();
    if botoes.is_empty() {
        String::new()
    } else {
        format!(r#"<p class="s-acoes">{botoes}</p>"#)
    }
}

fn imagem(midia: Option<&Midia>, sizes: &str) -> String {
    midia
        .map(|midia| renderizar_midia(midia, Papel::Conteudo, sizes))
        .unwrap_or_default()
}

fn lista<T>(tag: &str, classe: &str, itens: &[T], item: impl Fn(&T) -> String) -> String {
    let itens: String = itens.iter().map(item).collect();
    format!(r#"<{tag} class="{classe}">{itens}</{tag}>"#)
}

fn secao(classe: &str, titulo: Option<&str>, intro: Option<&str>, miolo: &str) -> String {
    let titulo = titulo
        .map(|titulo| format!("<h2>{}</h2>", marcacao(titulo)))
        .unwrap_or_default();
    let intro = intro
        .map(|intro| format!(r#"<p class="s-intro">{}</p>"#, marcacao(intro)))
        .unwrap_or_default();
    format!(r#"<section class="secao {classe}">{titulo}{intro}{miolo}</section>"#)
}

fn so_digitos(texto: &str) -> String {
    texto.chars().filter(char::is_ascii_digit).collect()
}

/// O número com o código do Brasil na frente, para `tel:` e `wa.me`.
fn com_codigo_do_pais(numero: &str) -> String {
    let digitos = so_digitos(numero);
    if digitos.len() <= 11 {
        format!("55{digitos}")
    } else {
        digitos
    }
}

pub(crate) fn email_valido(email: &str) -> bool {
    let email = email.trim();
    !email.contains(char::is_whitespace)
        && email
            .split_once('@')
            .is_some_and(|(nome, dominio)| !nome.is_empty() && dominio.contains('.'))
}

/// O topo de uma página de apresentação, com o `<h1>`. `None` quando a
/// página não tem abertura: aí o título sai do jeito de sempre.
pub fn renderizar_abertura(pagina: &Pagina) -> Option<String> {
    let abertura = pagina.abertura.as_ref()?;
    let sobretitulo = preenchido(&abertura.sobretitulo)
        .map(|texto| format!(r#"<p class="s-sobretitulo">{}</p>"#, marcacao(texto)))
        .unwrap_or_default();
    // A capa é a imagem que o visitante vê primeiro: carrega com prioridade.
    let capa = pagina
        .capa
        .as_ref()
        .map(|capa| renderizar_midia(capa, Papel::Principal, SIZES_DE_METADE))
        .filter(|html| !html.is_empty());
    let (classe, capa) = match capa {
        Some(html) => (
            "abertura com-imagem",
            format!(r#"<div class="s-imagem">{html}</div>"#),
        ),
        None => ("abertura", String::new()),
    };
    Some(format!(
        r#"<header class="{classe}"><div class="s-texto">{sobretitulo}<h1>{}</h1><p class="s-intro">{}</p>{}</div>{capa}</header>"#,
        marcacao(&pagina.titulo),
        marcacao(&abertura.texto),
        botoes(&abertura.acoes),
    ))
}

/// O HTML de uma seção. Vazio para bloco que não é seção.
/// O campo de um formulário: rótulo ligado ao controle, tipo que abre o
/// teclado certo no celular e o limite que o site também confere ao receber.
fn campo(campo: &Campo, formulario: usize, posicao: usize) -> String {
    let id = format!("f{formulario}c{posicao}");
    let obrigatorio = if campo.obrigatorio { " required" } else { "" };
    let limite = campo.tipo.limite();
    let controle = match campo.tipo {
        TipoDeCampo::Mensagem => format!(
            r#"<textarea id="{id}" name="c{posicao}" rows="5" maxlength="{limite}"{obrigatorio}></textarea>"#
        ),
        tipo => {
            let (tipo, completar) = match tipo {
                TipoDeCampo::Email => ("email", "email"),
                TipoDeCampo::Telefone => ("tel", "tel"),
                _ => ("text", "on"),
            };
            format!(
                r#"<input id="{id}" name="c{posicao}" type="{tipo}" autocomplete="{completar}" maxlength="{limite}"{obrigatorio}>"#
            )
        }
    };
    format!(
        r#"<label for="{id}">{}</label>{controle}"#,
        marcacao(&campo.rotulo)
    )
}

/// O HTML de uma seção. Vazio para bloco que não é seção. `indice` é a
/// posição do bloco no corpo.
pub(crate) fn renderizar(bloco: &Bloco, indice: usize) -> String {
    match bloco {
        Bloco::Cartoes {
            titulo,
            texto,
            itens,
        } => secao(
            "s-cartoes",
            preenchido(titulo),
            preenchido(texto),
            &lista("ul", "s-grade", itens, |item| {
                format!(
                    r#"<li class="s-cartao">{}<h3>{}</h3><p>{}</p>{}</li>"#,
                    imagem(item.midia.as_ref(), SIZES_DE_CARTAO),
                    marcacao(&item.titulo),
                    marcacao(&item.texto),
                    item.acao
                        .as_ref()
                        .map(|acao| botao(acao, "s-link"))
                        .unwrap_or_default(),
                )
            }),
        ),
        Bloco::Destaque {
            titulo,
            texto,
            midia,
            invertido,
            acao,
        } => {
            let classe = if *invertido {
                "secao s-destaque invertido"
            } else {
                "secao s-destaque"
            };
            format!(
                r#"<section class="{classe}"><div class="s-imagem">{}</div><div class="s-texto"><h2>{}</h2>{}{}</div></section>"#,
                imagem(Some(midia), SIZES_DE_METADE),
                marcacao(titulo),
                paragrafos(texto),
                botoes(acao.as_slice()),
            )
        }
        Bloco::Depoimentos { titulo, itens } => secao(
            "s-depoimentos",
            preenchido(titulo),
            None,
            &lista("ul", "s-grade", itens, |item| {
                let cargo = preenchido(&item.cargo).map(marcacao).unwrap_or_default();
                format!(
                    r#"<li><figure class="s-depoimento"><blockquote><p>{}</p></blockquote><figcaption>{}<span><strong>{}</strong>{cargo}</span></figcaption></figure></li>"#,
                    marcacao(&item.texto),
                    imagem(item.foto.as_ref(), SIZES_DE_FOTO),
                    marcacao(&item.autor),
                )
            }),
        ),
        Bloco::Numeros { titulo, itens } => secao(
            "s-numeros-secao",
            preenchido(titulo),
            None,
            &lista("dl", "s-numeros", itens, |item| {
                format!(
                    "<div><dt>{}</dt><dd>{}</dd></div>",
                    marcacao(&item.rotulo),
                    marcacao(&item.valor)
                )
            }),
        ),
        Bloco::Passos { titulo, itens } => secao(
            "s-passos-secao",
            preenchido(titulo),
            None,
            &lista("ol", "s-passos", itens, |item| {
                format!(
                    "<li><h3>{}</h3><p>{}</p></li>",
                    marcacao(&item.titulo),
                    marcacao(&item.texto)
                )
            }),
        ),
        Bloco::Planos { titulo, itens } => secao(
            "s-planos",
            preenchido(titulo),
            None,
            &lista("ul", "s-grade", itens, |plano| {
                let classe = if plano.destaque {
                    "s-plano recomendado"
                } else {
                    "s-plano"
                };
                let periodo = preenchido(&plano.periodo)
                    .map(|periodo| format!(" <span>{}</span>", marcacao(periodo)))
                    .unwrap_or_default();
                let descricao = preenchido(&plano.descricao)
                    .map(|descricao| format!("<p>{}</p>", marcacao(descricao)))
                    .unwrap_or_default();
                let inclui = if plano.itens.is_empty() {
                    String::new()
                } else {
                    lista("ul", "s-inclui", &plano.itens, |item| {
                        format!("<li>{}</li>", marcacao(item))
                    })
                };
                format!(
                    r#"<li class="{classe}"><h3>{}</h3><p class="s-preco"><strong>{}</strong>{periodo}</p>{descricao}{inclui}{}</li>"#,
                    marcacao(&plano.nome),
                    marcacao(&plano.preco),
                    botao(&plano.acao, "botao"),
                )
            }),
        ),
        Bloco::Galeria { titulo, itens } => secao(
            "s-galeria-secao",
            preenchido(titulo),
            None,
            &lista("ul", "s-galeria", itens, |midia| {
                format!("<li>{}</li>", imagem(Some(midia), SIZES_DE_CARTAO))
            }),
        ),
        Bloco::Logos { titulo, itens } => secao(
            "s-logos-secao",
            preenchido(titulo),
            None,
            &lista("ul", "s-logos", itens, |midia| {
                format!("<li>{}</li>", imagem(Some(midia), SIZES_DE_LOGO))
            }),
        ),
        Bloco::Faixa {
            titulo,
            texto,
            acoes,
        } => secao(
            "s-faixa",
            Some(titulo.as_str()),
            None,
            &format!(
                "{}{}",
                preenchido(texto)
                    .map(|texto| format!("<p>{}</p>", marcacao(texto)))
                    .unwrap_or_default(),
                botoes(acoes)
            ),
        ),
        Bloco::Formulario {
            titulo,
            texto,
            campos,
            botao,
            ..
        } => {
            let controles: String = campos
                .iter()
                .enumerate()
                .map(|(posicao, item)| campo(item, indice, posicao))
                .collect();
            // Sem `action`: o envio vai para o endereço da própria página. O
            // campo `site` é a isca: gente não o vê, robô o preenche.
            secao(
                "s-formulario",
                preenchido(titulo),
                preenchido(texto),
                &format!(
                    r#"<form method="post" id="formulario-{indice}"><input type="hidden" name="formulario" value="{indice}"><p class="s-isca" aria-hidden="true"><label for="f{indice}site">Não preencha</label><input id="f{indice}site" name="site" tabindex="-1" autocomplete="off"></p>{controles}<button class="botao" type="submit">{}</button></form>"#,
                    marcacao(botao)
                ),
            )
        }
        Bloco::Contato {
            titulo,
            texto,
            telefone,
            whatsapp,
            email,
            endereco,
            horario,
        } => {
            let canal =
                |rotulo: &str, valor: String| format!("<li><span>{rotulo}</span>{valor}</li>");
            let link = |destino: String, texto: &str| {
                format!(
                    r#"<a href="{}">{}</a>"#,
                    marcacao(&destino),
                    marcacao(texto)
                )
            };
            let mut canais = String::new();
            if let Some(numero) = preenchido(telefone) {
                let destino = format!("tel:+{}", com_codigo_do_pais(numero));
                canais.push_str(&canal("Telefone", link(destino, numero)));
            }
            if let Some(numero) = preenchido(whatsapp) {
                let destino = format!("https://wa.me/{}", com_codigo_do_pais(numero));
                canais.push_str(&canal("WhatsApp", link(destino, numero)));
            }
            if let Some(email) = preenchido(email).filter(|email| email_valido(email)) {
                canais.push_str(&canal("E-mail", link(format!("mailto:{email}"), email)));
            }
            if let Some(endereco) = preenchido(endereco) {
                canais.push_str(&canal("Endereço", marcacao(endereco)));
            }
            if let Some(horario) = preenchido(horario) {
                canais.push_str(&canal("Horário", marcacao(horario)));
            }
            secao(
                "s-contato",
                preenchido(titulo),
                preenchido(texto),
                &format!(r#"<ul class="s-canais">{canais}</ul>"#),
            )
        }
        _ => String::new(),
    }
}

fn acao_em_markdown(acao: &Acao) -> String {
    format!("[{}]({})", uma_linha(&acao.rotulo), acao.url)
}

/// A seção em Markdown, para o `llms-full.txt` e para a contagem de
/// palavras. Vazio para bloco que não é seção.
pub(crate) fn em_markdown(bloco: &Bloco) -> String {
    let mut linhas: Vec<String> = Vec::new();
    let titulo_da_secao = |titulo: Option<&str>| {
        titulo
            .map(|titulo| format!("## {}", uma_linha(titulo)))
            .into_iter()
            .collect::<Vec<_>>()
    };
    match bloco {
        Bloco::Cartoes {
            titulo,
            texto,
            itens,
        } => {
            linhas.extend(titulo_da_secao(preenchido(titulo)));
            linhas.extend(preenchido(texto).map(uma_linha));
            linhas.extend(itens.iter().map(|item| {
                let acao = item
                    .acao
                    .as_ref()
                    .map(|acao| format!(" {}", acao_em_markdown(acao)))
                    .unwrap_or_default();
                format!(
                    "- **{}**: {}{acao}",
                    uma_linha(&item.titulo),
                    uma_linha(&item.texto)
                )
            }));
        }
        Bloco::Destaque {
            titulo,
            texto,
            acao,
            ..
        } => {
            linhas.extend(titulo_da_secao(Some(titulo.as_str())));
            linhas.push(uma_linha(texto));
            linhas.extend(acao.as_ref().map(acao_em_markdown));
        }
        Bloco::Depoimentos { titulo, itens } => {
            linhas.extend(titulo_da_secao(preenchido(titulo)));
            linhas.extend(itens.iter().map(|item| {
                let cargo = preenchido(&item.cargo)
                    .map(|cargo| format!(", {}", uma_linha(cargo)))
                    .unwrap_or_default();
                format!(
                    "> {}\n> — {}{cargo}",
                    uma_linha(&item.texto),
                    uma_linha(&item.autor)
                )
            }));
        }
        Bloco::Numeros { titulo, itens } => {
            linhas.extend(titulo_da_secao(preenchido(titulo)));
            linhas.extend(
                itens.iter().map(|item| {
                    format!("- {}: {}", uma_linha(&item.rotulo), uma_linha(&item.valor))
                }),
            );
        }
        Bloco::Passos { titulo, itens } => {
            linhas.extend(titulo_da_secao(preenchido(titulo)));
            linhas.extend(itens.iter().enumerate().map(|(i, item)| {
                format!(
                    "{}. **{}**: {}",
                    i + 1,
                    uma_linha(&item.titulo),
                    uma_linha(&item.texto)
                )
            }));
        }
        Bloco::Planos { titulo, itens } => {
            linhas.extend(titulo_da_secao(preenchido(titulo)));
            linhas.extend(itens.iter().map(|plano| {
                let periodo = preenchido(&plano.periodo)
                    .map(|periodo| format!(" {}", uma_linha(periodo)))
                    .unwrap_or_default();
                let mut partes = vec![format!(
                    "- **{}**: {}{periodo}",
                    uma_linha(&plano.nome),
                    uma_linha(&plano.preco)
                )];
                partes.extend(preenchido(&plano.descricao).map(uma_linha));
                partes.extend(plano.itens.iter().map(|item| uma_linha(item)));
                partes.join("; ")
            }));
        }
        Bloco::Galeria { titulo, itens } | Bloco::Logos { titulo, itens } => {
            linhas.extend(titulo_da_secao(preenchido(titulo)));
            linhas.extend(
                itens
                    .iter()
                    .map(|midia| format!("- {}", uma_linha(&midia.alt))),
            );
        }
        Bloco::Formulario {
            titulo,
            texto,
            campos,
            ..
        } => {
            linhas.extend(titulo_da_secao(preenchido(titulo)));
            linhas.extend(preenchido(texto).map(uma_linha));
            linhas.extend(
                campos
                    .iter()
                    .map(|campo| format!("- {}", uma_linha(&campo.rotulo))),
            );
        }
        Bloco::Faixa {
            titulo,
            texto,
            acoes,
        } => {
            linhas.extend(titulo_da_secao(Some(titulo.as_str())));
            linhas.extend(preenchido(texto).map(uma_linha));
            linhas.extend(acoes.iter().map(acao_em_markdown));
        }
        Bloco::Contato {
            titulo,
            texto,
            telefone,
            whatsapp,
            email,
            endereco,
            horario,
        } => {
            linhas.extend(titulo_da_secao(preenchido(titulo)));
            linhas.extend(preenchido(texto).map(uma_linha));
            for (rotulo, valor) in [
                ("Telefone", telefone),
                ("WhatsApp", whatsapp),
                ("E-mail", email),
                ("Endereço", endereco),
                ("Horário", horario),
            ] {
                linhas.extend(
                    preenchido(valor).map(|valor| format!("- {rotulo}: {}", uma_linha(valor))),
                );
            }
        }
        _ => {}
    }
    linhas.join("\n")
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::demonstracao;
    use crate::tipos::{Cartao, Conteudo, Numero};

    fn apresentacao() -> Pagina {
        match demonstracao::apresentacao().conteudo {
            Conteudo::Pagina(pagina) => pagina,
            _ => unreachable!("a apresentação é uma página"),
        }
    }

    #[test]
    fn abertura_leva_o_h1_e_a_capa_com_prioridade() {
        let html = renderizar_abertura(&apresentacao()).expect("abertura");
        assert_eq!(html.matches("<h1>").count(), 1);
        assert!(html.contains(r#"class="abertura com-imagem""#));
        assert!(html.contains(r#"fetchpriority="high""#));
        assert!(html.contains(r#"<a class="botao" href="/contato">"#));
        assert!(html.contains(r#"<a class="botao secundario" href="/servicos">"#));
    }

    #[test]
    fn pagina_sem_abertura_nao_tem_topo_proprio() {
        let mut pagina = apresentacao();
        pagina.abertura = None;
        assert_eq!(renderizar_abertura(&pagina), None);
    }

    #[test]
    fn secao_nunca_gera_h1_e_escapa_o_que_recebe() {
        let pagina = apresentacao();
        let html: String = pagina
            .corpo
            .iter()
            .enumerate()
            .map(|(indice, bloco)| renderizar(bloco, indice))
            .collect();
        assert!(!html.contains("<h1"));
        assert_eq!(
            html.matches("<section").count(),
            pagina.corpo.iter().filter(|b| b.eh_secao()).count()
        );

        let malicioso = Bloco::Cartoes {
            titulo: Some("<script>alert(1)</script>".into()),
            texto: None,
            itens: vec![Cartao {
                titulo: "\"aspas\"".into(),
                texto: "a < b".into(),
                midia: None,
                acao: Some(Acao {
                    rotulo: "Abrir".into(),
                    url: "javascript:alert(1)".into(),
                }),
            }],
        };
        let html = renderizar(&malicioso, 0);
        assert!(!html.contains("<script>"));
        assert!(!html.contains("javascript:"));
        assert!(html.contains("a &lt; b"));
    }

    #[test]
    fn contato_monta_os_links_de_cada_canal() {
        let html = renderizar(
            &Bloco::Contato {
                titulo: Some("Fale com a gente".into()),
                texto: None,
                telefone: Some("(11) 4000-0000".into()),
                whatsapp: Some("(11) 90000-0000".into()),
                email: Some("contato@exemplo.example".into()),
                endereco: Some("Rua das Flores, 100".into()),
                horario: None,
            },
            0,
        );
        assert!(html.contains(r#"href="tel:+551140000000""#));
        assert!(html.contains(r#"href="https://wa.me/5511900000000""#));
        assert!(html.contains(r#"href="mailto:contato@exemplo.example""#));
        assert!(html.contains("Rua das Flores, 100"));
        assert!(!html.contains("Horário"));
    }

    #[test]
    fn formulario_sai_sem_script_com_rotulo_tipo_e_isca() {
        let html = renderizar(
            &Bloco::Formulario {
                titulo: Some("Fale com a gente".into()),
                texto: None,
                campos: vec![
                    Campo {
                        rotulo: "Seu <nome>".into(),
                        tipo: TipoDeCampo::Texto,
                        obrigatorio: true,
                    },
                    Campo {
                        rotulo: "E-mail".into(),
                        tipo: TipoDeCampo::Email,
                        obrigatorio: false,
                    },
                    Campo {
                        rotulo: "Mensagem".into(),
                        tipo: TipoDeCampo::Mensagem,
                        obrigatorio: true,
                    },
                ],
                botao: "Enviar".into(),
                agradecimento: "Recebemos.".into(),
            },
            3,
        );
        assert!(html.contains(r#"<form method="post" id="formulario-3">"#));
        assert!(html.contains(r#"name="formulario" value="3""#));
        assert!(html.contains(r#"<label for="f3c0">Seu &lt;nome&gt;</label>"#));
        assert!(
            html.contains(r#"name="c0" type="text" autocomplete="on" maxlength="200" required>"#)
        );
        assert!(html.contains(r#"name="c1" type="email" autocomplete="email" maxlength="200">"#));
        assert!(
            html.contains(r#"<textarea id="f3c2" name="c2" rows="5" maxlength="4000" required>"#)
        );
        // A isca existe, fora da ordem de tabulação.
        assert!(html.contains(r#"name="site" tabindex="-1""#));
        assert!(!html.contains("<script") && !html.contains("action="));
    }

    #[test]
    fn markdown_da_secao_traz_titulo_e_itens() {
        let texto = em_markdown(&Bloco::Numeros {
            titulo: Some("Em números".into()),
            itens: vec![Numero {
                valor: "15 anos".into(),
                rotulo: "de oficina".into(),
            }],
        });
        assert_eq!(texto, "## Em números\n- de oficina: 15 anos");
        assert_eq!(
            em_markdown(&Bloco::Titulo {
                nivel: 2,
                texto: "Não é seção".into()
            }),
            ""
        );
    }
}
