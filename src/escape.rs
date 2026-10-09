//! Escape de texto de quem edita. Nada do conteúdo chega à saída sem passar
//! por aqui.

use serde_json::Value;

/// Serve para texto e para valor de atributo entre aspas, em HTML e em XML.
pub(crate) fn marcacao(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    for caractere in texto.chars() {
        match caractere {
            '&' => saida.push_str("&amp;"),
            '<' => saida.push_str("&lt;"),
            '>' => saida.push_str("&gt;"),
            '"' => saida.push_str("&quot;"),
            '\'' => saida.push_str("&#39;"),
            _ => saida.push(caractere),
        }
    }
    saida
}

/// JSON pronto para ficar dentro de `<script>`: `<`, `>` e `&` viram `\u`,
/// para o texto de quem edita não fechar a tag.
pub(crate) fn json_para_script(valor: &Value) -> String {
    let mut saida = String::new();
    for caractere in valor.to_string().chars() {
        match caractere {
            '<' => saida.push_str("\\u003c"),
            '>' => saida.push_str("\\u003e"),
            '&' => saida.push_str("\\u0026"),
            '\u{2028}' => saida.push_str("\\u2028"),
            '\u{2029}' => saida.push_str("\\u2029"),
            _ => saida.push(caractere),
        }
    }
    saida
}

/// Texto que precisa caber em uma linha de Markdown.
pub(crate) fn uma_linha(texto: &str) -> String {
    texto.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    #[test]
    fn marcacao_escapa_os_cinco_caracteres() {
        assert_eq!(
            marcacao(r#"<a href="x">Tom & 'Jerry'</a>"#),
            "&lt;a href=&quot;x&quot;&gt;Tom &amp; &#39;Jerry&#39;&lt;/a&gt;"
        );
    }

    #[test]
    fn json_nao_fecha_o_script() {
        let saida = json_para_script(&json!({ "nome": "</script><script>alert(1)" }));
        assert!(!saida.contains('<'));
        assert!(saida.contains("\\u003c/script\\u003e"));
    }

    #[test]
    fn uma_linha_junta_quebras() {
        assert_eq!(uma_linha("a\n b\r\n  c"), "a b c");
    }
}
