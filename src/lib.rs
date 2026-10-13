//! Regras de conteúdo compartilhadas pelo CMS e pelo Lojas da Ávila Ops.
//!
//! O crate recebe conteúdo e devolve os problemas que impedem a publicação e
//! os artefatos da página: cabeçalho, JSON-LD, HTML do corpo e das imagens,
//! sitemaps, `robots.txt` e `llms.txt`. Não tem banco, rede, relógio nem
//! estado global: data e hora entram por parâmetro.

pub mod conteudo;
pub mod cor;
pub mod demonstracao;
pub mod descoberta;
#[cfg(feature = "imagem")]
pub mod imagem;
pub mod midia;
pub mod schema;
pub mod secoes;
pub mod seo;
pub mod tipos;
pub mod validacao;

mod escape;
mod url;
