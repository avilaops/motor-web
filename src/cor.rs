//! A cor de destaque: a da marca do site e a de uma página. É usada como
//! texto sobre fundo branco (links) e como fundo de texto branco (botões):
//! nos dois casos o contraste que conta é o dela contra o branco.

use std::fmt;

/// O contraste mínimo de texto normal contra o fundo, pela WCAG.
const CONTRASTE_MINIMO: f64 = 4.5;

/// Por que uma cor de destaque não serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorRecusada {
    Formato,
    Clara,
}

impl fmt::Display for CorRecusada {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Formato => {
                "Escreva a cor como #225cf2: cerquilha e seis dígitos de 0 a 9 ou de a até f."
            }
            Self::Clara => {
                "Esta cor é clara demais: o texto dos links e dos botões ficaria difícil de ler. Escolha um tom mais escuro."
            }
        })
    }
}

impl std::error::Error for CorRecusada {}

/// A luminância relativa de uma cor, de 0 (preto) a 1 (branco).
fn luminancia(cor: [u8; 3]) -> f64 {
    let canal = |valor: u8| {
        let v = f64::from(valor) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * canal(cor[0]) + 0.7152 * canal(cor[1]) + 0.0722 * canal(cor[2])
}

/// Confere a cor de destaque e a devolve em minúsculas.
pub fn conferir_cor(cor: &str) -> Result<String, CorRecusada> {
    let cor = cor.trim().to_ascii_lowercase();
    let digitos = cor.strip_prefix('#').ok_or(CorRecusada::Formato)?;
    if digitos.len() != 6 || !digitos.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CorRecusada::Formato);
    }
    let canal = |i: usize| u8::from_str_radix(&digitos[i..i + 2], 16).unwrap_or(0);
    let contraste = 1.05 / (luminancia([canal(0), canal(2), canal(4)]) + 0.05);
    if contraste < CONTRASTE_MINIMO {
        return Err(CorRecusada::Clara);
    }
    Ok(cor)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cor_de_destaque_precisa_ser_legivel() {
        assert_eq!(conferir_cor(" #225CF2 "), Ok("#225cf2".to_string()));
        assert_eq!(conferir_cor("#9a3412"), Ok("#9a3412".to_string()));
        for torta in ["225cf2", "#225cf", "#225cfg", "azul", "", "#ééééé"] {
            assert_eq!(conferir_cor(torta), Err(CorRecusada::Formato), "{torta}");
        }
        // Amarelo e azul claro não seguram texto branco nem leem sobre branco.
        for clara in ["#f4c544", "#7fb2ff", "#ffffff"] {
            assert_eq!(conferir_cor(clara), Err(CorRecusada::Clara), "{clara}");
        }
    }
}
