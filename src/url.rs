//! Leitura mínima de endereços. Só o que as regras precisam: saber se um
//! link é aceitável e achar o endereço de incorporação de um vídeo.

/// Link aceito no conteúdo: `http(s)`, caminho do próprio site ou `mailto:`
/// com um endereço de e-mail.
pub(crate) fn link_valido(endereco: &str) -> bool {
    endereco_web_valido(endereco) || mailto_valido(endereco)
}

/// Endereço que um navegador abre: `http(s)` ou caminho do próprio site.
pub(crate) fn endereco_web_valido(endereco: &str) -> bool {
    if endereco.is_empty()
        || endereco
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
    {
        return false;
    }
    if endereco.starts_with('/') {
        return !endereco.starts_with("//");
    }
    partes(endereco).is_some_and(|(host, _)| !host.is_empty())
}

/// Endereço absoluto `https://` com host, como o que vai numa canônica.
pub(crate) fn endereco_absoluto_valido(endereco: &str) -> bool {
    endereco.to_ascii_lowercase().starts_with("https://")
        && endereco_web_valido(endereco)
        && partes(endereco).is_some_and(|(host, _)| host.contains('.'))
}

/// `mailto:` com um único destinatário, `nome@dominio.tld`, e assunto
/// opcional depois de `?`. Sem espaço nem caractere de controle: o resto
/// do texto vai escapado para o `href`.
fn mailto_valido(endereco: &str) -> bool {
    let Some(resto) = endereco
        .get(..7)
        .filter(|inicio| inicio.eq_ignore_ascii_case("mailto:"))
        .map(|_| &endereco[7..])
    else {
        return false;
    };
    if resto.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let destinatario = resto.split('?').next().unwrap_or("");
    let Some((usuario, dominio)) = destinatario.split_once('@') else {
        return false;
    };
    !usuario.is_empty()
        && !dominio.contains('@')
        && !dominio.contains(',')
        && !usuario.contains(',')
        && dominio.split('.').all(|parte| {
            !parte.is_empty() && parte.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        && dominio.contains('.')
}

/// Endereço de incorporação de um vídeo do YouTube ou do Vimeo.
pub(crate) fn incorporacao_de_video(endereco: &str) -> Option<String> {
    let (host, resto) = partes(endereco)?;
    let host = host.strip_prefix("www.").unwrap_or(host.as_str());
    let caminho = resto.split(['?', '#']).next().unwrap_or("");
    let segmentos: Vec<&str> = caminho.split('/').filter(|s| !s.is_empty()).collect();

    match host {
        "youtu.be" => {
            let id = identificador(segmentos.first()?)?;
            Some(youtube(id))
        }
        "youtube.com" | "m.youtube.com" | "youtube-nocookie.com" => match segmentos.as_slice() {
            ["watch"] => {
                let consulta = resto.split('#').next()?.split_once('?')?.1;
                let id = consulta
                    .split('&')
                    .find_map(|par| par.strip_prefix("v="))
                    .and_then(identificador)?;
                Some(youtube(id))
            }
            ["embed" | "shorts" | "live", id] => Some(youtube(identificador(id)?)),
            _ => None,
        },
        "vimeo.com" => Some(vimeo(numero(segmentos.first()?)?)),
        "player.vimeo.com" => match segmentos.as_slice() {
            ["video", id] => Some(vimeo(numero(id)?)),
            _ => None,
        },
        _ => None,
    }
}

/// Separa host (em minúsculas, sem porta) e o resto de um endereço `http(s)`.
fn partes(endereco: &str) -> Option<(String, &str)> {
    let minusculo = endereco.to_ascii_lowercase();
    let inicio = if minusculo.starts_with("https://") {
        8
    } else if minusculo.starts_with("http://") {
        7
    } else {
        return None;
    };
    let depois = &endereco[inicio..];
    let fim = depois.find(['/', '?', '#']).unwrap_or(depois.len());
    let autoridade = &depois[..fim];
    // `usuario@host` é a forma clássica de disfarçar o destino de um link.
    if autoridade.contains('@') {
        return None;
    }
    let host = autoridade
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    Some((host, &depois[fim..]))
}

fn identificador(texto: &str) -> Option<&str> {
    let valido = !texto.is_empty()
        && texto.len() <= 32
        && texto
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    valido.then_some(texto)
}

fn numero(texto: &str) -> Option<&str> {
    let valido =
        !texto.is_empty() && texto.len() <= 20 && texto.chars().all(|c| c.is_ascii_digit());
    valido.then_some(texto)
}

fn youtube(id: &str) -> String {
    format!("https://www.youtube-nocookie.com/embed/{id}")
}

fn vimeo(id: &str) -> String {
    format!("https://player.vimeo.com/video/{id}")
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn aceita_http_e_caminho_do_site() {
        assert!(link_valido("https://exemplo.com.br/a?b=1"));
        assert!(link_valido("http://exemplo.com.br"));
        assert!(link_valido("/contato"));
    }

    #[test]
    fn recusa_o_resto() {
        for ruim in [
            "",
            "javascript:alert(1)",
            "//outro.com",
            "data:text/html,x",
            "https://",
            "https://a b.com",
            "contato",
            "https://banco.com@golpe.com/",
        ] {
            assert!(!link_valido(ruim), "{ruim}");
        }
    }

    #[test]
    fn aceita_mailto_com_um_destinatario() {
        for bom in [
            "mailto:comercial@exemplo.com.br",
            "MAILTO:contato@exemplo.com",
            "mailto:contato@exemplo.com?subject=Or%C3%A7amento",
        ] {
            assert!(link_valido(bom), "{bom}");
        }
        for ruim in [
            "mailto:",
            "mailto:contato",
            "mailto:@exemplo.com",
            "mailto:contato@",
            "mailto:contato@exemplo",
            "mailto:a@exemplo.com,b@exemplo.com",
            "mailto:contato@exemplo.com?subject=Oi tudo",
            "mailto:contato@exe<mplo.com",
            "mailto:a@b@exemplo.com",
        ] {
            assert!(!link_valido(ruim), "{ruim}");
        }
    }

    #[test]
    fn endereco_web_nao_aceita_mailto() {
        assert!(endereco_web_valido("/contato"));
        assert!(!endereco_web_valido("mailto:contato@exemplo.com"));
    }

    #[test]
    fn canonica_so_com_https_absoluto() {
        assert!(endereco_absoluto_valido("https://exemplo.com.br/cms"));
        for ruim in [
            "http://exemplo.com.br/cms",
            "/cms",
            "https://localhost/cms",
            "mailto:contato@exemplo.com",
            "https://exemplo.com.br/a b",
        ] {
            assert!(!endereco_absoluto_valido(ruim), "{ruim}");
        }
    }

    #[test]
    fn acha_a_incorporacao_do_youtube() {
        let esperado = Some("https://www.youtube-nocookie.com/embed/abc_DEF-123".to_string());
        for endereco in [
            "https://www.youtube.com/watch?v=abc_DEF-123",
            "https://youtube.com/watch?t=10&v=abc_DEF-123#x",
            "https://youtu.be/abc_DEF-123?si=xyz",
            "https://www.youtube.com/embed/abc_DEF-123",
            "https://www.youtube.com/shorts/abc_DEF-123",
        ] {
            assert_eq!(incorporacao_de_video(endereco), esperado, "{endereco}");
        }
    }

    #[test]
    fn acha_a_incorporacao_do_vimeo() {
        let esperado = Some("https://player.vimeo.com/video/123456".to_string());
        assert_eq!(incorporacao_de_video("https://vimeo.com/123456"), esperado);
        assert_eq!(
            incorporacao_de_video("https://player.vimeo.com/video/123456"),
            esperado
        );
    }

    #[test]
    fn recusa_video_de_outra_origem() {
        for ruim in [
            "https://exemplo.com/video.mp4",
            "https://youtube.com.golpe.com/watch?v=abc",
            "https://www.youtube.com/watch?v=\"><script>",
            "https://vimeo.com/canal",
            "youtube.com/watch?v=abc",
        ] {
            assert_eq!(incorporacao_de_video(ruim), None, "{ruim}");
        }
    }
}
