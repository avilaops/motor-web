//! Contrato do conteúdo. É o que o CMS grava e entrega, e o que o Lojas
//! mapeia a partir do catálogo dele.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Site {
    /// `https://exemplo.com.br`, sem barra final.
    pub origem: String,
    pub nome: String,
    /// Contexto da marca; abre o `llms.txt`.
    pub descricao: String,
    /// `pt-BR`.
    pub idioma: String,
    pub logo: Midia,
    pub organizacao: Organizacao,
    #[serde(default)]
    pub diretrizes_ia: Vec<String>,
    /// Peso máximo de uma página, em kB.
    pub orcamento_peso_kb: u32,
}

impl Site {
    pub(crate) fn origem_limpa(&self) -> &str {
        self.origem.trim_end_matches('/')
    }

    /// Endereço completo de um caminho ou de um arquivo do site.
    pub fn url_absoluta(&self, endereco: &str) -> String {
        if endereco.starts_with("http://") || endereco.starts_with("https://") {
            endereco.to_string()
        } else if endereco.starts_with('/') {
            format!("{}{}", self.origem_limpa(), endereco)
        } else {
            format!("{}/{}", self.origem_limpa(), endereco)
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Organizacao {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub razao_social: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telefone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endereco: Option<Endereco>,
    /// Vira `sameAs`.
    #[serde(default)]
    pub perfis: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Endereco {
    pub logradouro: String,
    pub cidade: String,
    pub uf: String,
    pub cep: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Midia {
    pub id: String,
    pub alt: String,
    pub largura: u32,
    pub altura: u32,
    #[serde(default)]
    pub variantes: Vec<Variante>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legenda: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credito: Option<String>,
    #[serde(default, skip_serializing_if = "Direitos::vazio")]
    pub direitos: Direitos,
}

/// De quem é a imagem e em que termos ela pode ser usada. É o que a busca de
/// imagens lê para mostrar o crédito e o selo de imagem licenciável.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Direitos {
    /// Quem fez a imagem.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoria: Option<String>,
    /// O aviso de direitos autorais, como "© 2026 Oficina Exemplo".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aviso: Option<String>,
    /// O endereço da licença de uso: `https://` ou um caminho do site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub licenca: Option<String>,
    /// O endereço onde se pede a licença.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aquisicao: Option<String>,
}

impl Direitos {
    pub fn vazio(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Variante {
    pub formato: Formato,
    pub largura: u32,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Formato {
    Avif,
    Webp,
}

impl Formato {
    pub fn extensao(self) -> &'static str {
        match self {
            Formato::Avif => "avif",
            Formato::Webp => "webp",
        }
    }

    pub fn tipo_mime(self) -> &'static str {
        match self {
            Formato::Avif => "image/avif",
            Formato::Webp => "image/webp",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Seo {
    pub titulo: String,
    pub descricao: String,
    pub indexar: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imagem_social: Option<Midia>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemTrilha {
    pub nome: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Autor {
    pub slug: String,
    pub nome: String,
    pub cargo: String,
    pub bio: String,
    pub foto: Midia,
    #[serde(default)]
    pub perfis: Vec<String>,
    #[serde(default)]
    pub credenciais: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Categoria {
    pub slug: String,
    pub nome: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pagina {
    /// Vazio na home.
    pub slug: String,
    pub titulo: String,
    #[serde(default)]
    pub corpo: Vec<Bloco>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capa: Option<Midia>,
    pub publicado_em: DateTime<Utc>,
    pub atualizado_em: DateTime<Utc>,
    pub seo: Seo,
    #[serde(default)]
    pub trilha: Vec<ItemTrilha>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TipoPost {
    Artigo,
    GuiaTecnico,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Post {
    pub tipo: TipoPost,
    pub slug: String,
    pub titulo: String,
    pub resumo: String,
    pub capa: Midia,
    #[serde(default)]
    pub corpo: Vec<Bloco>,
    pub autor: Autor,
    pub categoria: Categoria,
    #[serde(default)]
    pub tags: Vec<String>,
    pub publicado_em: DateTime<Utc>,
    pub atualizado_em: DateTime<Utc>,
    pub seo: Seo,
    #[serde(default)]
    pub trilha: Vec<ItemTrilha>,
}

/// Contrato de produto para SEO e dados estruturados. Preço, estoque e
/// cobrança continuam sendo do Lojas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Product {
    pub slug: String,
    pub nome: String,
    pub descricao: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marca: Option<String>,
    /// A primeira é a principal.
    #[serde(default)]
    pub imagens: Vec<Midia>,
    #[serde(default)]
    pub variacoes: Vec<Variacao>,
    /// Só avaliações do próprio site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avaliacoes: Option<Avaliacoes>,
    pub politica_devolucao_url: String,
    pub atualizado_em: DateTime<Utc>,
    pub seo: Seo,
    #[serde(default)]
    pub trilha: Vec<ItemTrilha>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Variacao {
    pub sku: String,
    pub nome: String,
    pub preco_centavos: i64,
    pub disponibilidade: Disponibilidade,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gtin: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Disponibilidade {
    EmEstoque,
    SemEstoque,
    SobEncomenda,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Avaliacoes {
    pub media: f32,
    pub total: u32,
}

/// O corpo é uma lista de blocos tipados. Não existe HTML livre, e o `<h1>`
/// não é bloco: sai do título do documento.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "tipo",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Bloco {
    Paragrafo {
        trechos: Vec<Trecho>,
    },
    /// Nível 2, 3 ou 4.
    Titulo {
        nivel: u8,
        texto: String,
    },
    Imagem {
        midia: Midia,
    },
    Lista {
        ordenada: bool,
        itens: Vec<String>,
    },
    Citacao {
        texto: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fonte: Option<String>,
    },
    Tabela {
        cabecalho: Vec<String>,
        linhas: Vec<Vec<String>>,
    },
    Perguntas {
        itens: Vec<Pergunta>,
    },
    /// Só YouTube e Vimeo.
    Video {
        url: String,
        titulo: String,
    },
    Chamada {
        texto: String,
        rotulo: String,
        url: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Trecho {
    pub texto: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub negrito: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub italico: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pergunta {
    pub pergunta: String,
    pub resposta: String,
}

/// Um documento e o caminho em que ele é servido. O caminho é de quem chama,
/// porque CMS e Lojas têm rotas diferentes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Documento {
    /// `/blog/como-escolher`. A home é `/`.
    pub caminho: String,
    pub conteudo: Conteudo,
}

// Um documento é tratado por vez; pôr as variantes em `Box` complicaria o
// contrato para economizar bytes que não fazem diferença aqui.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "especie", content = "dados", rename_all = "kebab-case")]
pub enum Conteudo {
    Pagina(Pagina),
    Post(Post),
    Produto(Product),
}

impl Documento {
    pub fn seo(&self) -> &Seo {
        match &self.conteudo {
            Conteudo::Pagina(p) => &p.seo,
            Conteudo::Post(p) => &p.seo,
            Conteudo::Produto(p) => &p.seo,
        }
    }

    /// O texto que vira o `<h1>`.
    pub fn titulo(&self) -> &str {
        match &self.conteudo {
            Conteudo::Pagina(p) => &p.titulo,
            Conteudo::Post(p) => &p.titulo,
            Conteudo::Produto(p) => &p.nome,
        }
    }

    pub fn slug(&self) -> &str {
        match &self.conteudo {
            Conteudo::Pagina(p) => &p.slug,
            Conteudo::Post(p) => &p.slug,
            Conteudo::Produto(p) => &p.slug,
        }
    }

    pub fn corpo(&self) -> &[Bloco] {
        match &self.conteudo {
            Conteudo::Pagina(p) => &p.corpo,
            Conteudo::Post(p) => &p.corpo,
            Conteudo::Produto(_) => &[],
        }
    }

    pub fn trilha(&self) -> &[ItemTrilha] {
        match &self.conteudo {
            Conteudo::Pagina(p) => &p.trilha,
            Conteudo::Post(p) => &p.trilha,
            Conteudo::Produto(p) => &p.trilha,
        }
    }

    pub fn atualizado_em(&self) -> DateTime<Utc> {
        match &self.conteudo {
            Conteudo::Pagina(p) => p.atualizado_em,
            Conteudo::Post(p) => p.atualizado_em,
            Conteudo::Produto(p) => p.atualizado_em,
        }
    }

    /// A imagem principal: capa da página ou do post, primeira do produto.
    pub fn capa(&self) -> Option<&Midia> {
        match &self.conteudo {
            Conteudo::Pagina(p) => p.capa.as_ref(),
            Conteudo::Post(p) => Some(&p.capa),
            Conteudo::Produto(p) => p.imagens.first(),
        }
    }

    /// Todas as imagens do documento, a principal primeiro.
    pub fn midias(&self) -> Vec<&Midia> {
        let mut midias: Vec<&Midia> = match &self.conteudo {
            Conteudo::Produto(p) => p.imagens.iter().collect(),
            _ => self.capa().into_iter().collect(),
        };
        for bloco in self.corpo() {
            if let Bloco::Imagem { midia } = bloco {
                midias.push(midia);
            }
        }
        midias
    }

    pub fn eh_home(&self) -> bool {
        matches!(&self.conteudo, Conteudo::Pagina(p) if p.slug.is_empty())
    }
}
