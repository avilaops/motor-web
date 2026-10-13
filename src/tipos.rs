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
    /// Não escrever o crédito embaixo da imagem (logo, ícone, ilustração da
    /// casa). O crédito e os direitos continuam no dado estruturado.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ocultar_credito: bool,
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

/// Um modelo 3D da biblioteca do site. O GLB é o que o visualizador abre na
/// página e o Android usa na realidade aumentada; o USDZ é o que o iPhone
/// pede para a mesma coisa. Sem USDZ, o iPhone fica só com o visualizador.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Modelo3d {
    pub id: String,
    pub glb: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usdz: Option<String>,
    /// O que o modelo mostra, para quem não o enxerga.
    pub descricao: String,
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
    /// O endereço oficial desta página, quando ele mora em outro lugar
    /// (`https://` completo). Vazio, a canônica é o próprio caminho.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonica: Option<String>,
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
    /// O nome curto da página no menu do site. Sem ele, vale o título.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu: Option<String>,
    /// A cor de destaque desta página (`#rrggbb`), no lugar da do site: a de
    /// um produto na página que o apresenta. Sem ela, vale a do site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destaque: Option<String>,
    #[serde(default)]
    pub corpo: Vec<Bloco>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capa: Option<Midia>,
    /// O topo de uma página de apresentação: o título ganha chamada, texto e
    /// botões, e a capa vira a imagem ao lado.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abertura: Option<Abertura>,
    pub publicado_em: DateTime<Utc>,
    pub atualizado_em: DateTime<Utc>,
    pub seo: Seo,
    #[serde(default)]
    pub trilha: Vec<ItemTrilha>,
}

/// O que acompanha o `<h1>` no topo de uma página de apresentação.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Abertura {
    /// A linha curta acima do título.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sobretitulo: Option<String>,
    pub texto: String,
    /// Até dois botões; o primeiro é o principal.
    #[serde(default)]
    pub acoes: Vec<Acao>,
}

/// Um botão: o que está escrito e para onde leva.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Acao {
    pub rotulo: String,
    pub url: String,
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

/// Um produto de catálogo: o que a empresa oferece, apresentado com ficha
/// técnica, fotos e modelo 3D. Não tem preço, estoque nem carrinho (isso é do
/// Lojas, no contrato [`Product`]): o botão leva a pessoa a pedir orçamento.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Item {
    pub slug: String,
    pub nome: String,
    /// O código do produto no catálogo da empresa, como `LD-A034`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codigo: Option<String>,
    /// Uma ou duas frases: aparecem no topo da página e no cartão da listagem.
    pub resumo: String,
    /// A primeira é a principal.
    #[serde(default)]
    pub imagens: Vec<Midia>,
    /// O modelo 3D do produto, no lugar de destaque da página.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modelo: Option<Modelo3d>,
    /// A ficha técnica, na ordem em que aparece.
    #[serde(default)]
    pub ficha: Vec<Caracteristica>,
    #[serde(default)]
    pub corpo: Vec<Bloco>,
    /// O botão da página: pedir orçamento, chamar no WhatsApp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acao: Option<Acao>,
    pub publicado_em: DateTime<Utc>,
    pub atualizado_em: DateTime<Utc>,
    pub seo: Seo,
    #[serde(default)]
    pub trilha: Vec<ItemTrilha>,
}

/// Uma linha da ficha técnica. A que é `filtro` também separa o catálogo:
/// a listagem oferece os valores dela para a pessoa escolher.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Caracteristica {
    pub nome: String,
    pub valor: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub filtro: bool,
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

/// O corpo é uma lista de blocos tipados: os de texto corrido e as seções.
/// Não existe HTML livre, e o `<h1>` não é bloco: sai do título do documento.
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
    /// Um modelo 3D que o visitante gira na página e, no celular, põe no
    /// próprio espaço. A capa é a imagem mostrada antes de o modelo carregar
    /// e para quem está sem o visualizador.
    #[serde(rename = "modelo-3d")]
    Modelo3d {
        modelo: Modelo3d,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capa: Option<Midia>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        legenda: Option<String>,
    },
    Chamada {
        texto: String,
        rotulo: String,
        url: String,
    },
    // Daqui para baixo, as seções: faixas de página inteira, com título de
    // nível 2 próprio. Uma página com seção sai no leiaute largo.
    /// Grade de cartões: serviços, recursos, diferenciais.
    Cartoes {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        titulo: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        texto: Option<String>,
        itens: Vec<Cartao>,
    },
    /// Imagem de um lado e texto do outro.
    Destaque {
        titulo: String,
        texto: String,
        midia: Midia,
        /// A imagem vai para a direita.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        invertido: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        acao: Option<Acao>,
    },
    Depoimentos {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        titulo: Option<String>,
        itens: Vec<Depoimento>,
    },
    /// Números de destaque: "15 anos", "1.200 clientes".
    Numeros {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        titulo: Option<String>,
        itens: Vec<Numero>,
    },
    /// Como funciona, em etapas numeradas.
    Passos {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        titulo: Option<String>,
        itens: Vec<Passo>,
    },
    /// Planos e preços, lado a lado.
    Planos {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        titulo: Option<String>,
        itens: Vec<Plano>,
    },
    Galeria {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        titulo: Option<String>,
        itens: Vec<Midia>,
    },
    /// Marcas de clientes ou parceiros, em linha.
    Logos {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        titulo: Option<String>,
        itens: Vec<Midia>,
    },
    /// Faixa de chamada no fim de uma página: título, texto e botões.
    Faixa {
        titulo: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        texto: Option<String>,
        acoes: Vec<Acao>,
    },
    /// Formulário de contato. O visitante preenche, o site guarda a mensagem
    /// e avisa o dono; não há script, o envio é o do próprio navegador.
    Formulario {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        titulo: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        texto: Option<String>,
        campos: Vec<Campo>,
        /// O que está escrito no botão de enviar.
        botao: String,
        /// O que o visitante lê depois de enviar.
        agradecimento: String,
    },
    /// Como falar com o negócio. Pelo menos um canal.
    Contato {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        titulo: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        texto: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        telefone: Option<String>,
        /// O número com DDD; o link sai com o código do Brasil.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        whatsapp: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        email: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        endereco: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        horario: Option<String>,
    },
}

impl Bloco {
    /// Seção ocupa a largura da página; os outros blocos são texto corrido.
    pub fn eh_secao(&self) -> bool {
        !matches!(
            self,
            Bloco::Paragrafo { .. }
                | Bloco::Titulo { .. }
                | Bloco::Imagem { .. }
                | Bloco::Lista { .. }
                | Bloco::Citacao { .. }
                | Bloco::Tabela { .. }
                | Bloco::Perguntas { .. }
                | Bloco::Video { .. }
                | Bloco::Modelo3d { .. }
                | Bloco::Chamada { .. }
        )
    }

    /// As imagens do bloco, na ordem em que aparecem.
    pub fn midias(&self) -> Vec<&Midia> {
        match self {
            Bloco::Imagem { midia } | Bloco::Destaque { midia, .. } => vec![midia],
            Bloco::Cartoes { itens, .. } => itens.iter().filter_map(|i| i.midia.as_ref()).collect(),
            Bloco::Depoimentos { itens, .. } => {
                itens.iter().filter_map(|i| i.foto.as_ref()).collect()
            }
            Bloco::Galeria { itens, .. } | Bloco::Logos { itens, .. } => itens.iter().collect(),
            Bloco::Modelo3d { capa, .. } => capa.iter().collect(),
            _ => Vec::new(),
        }
    }

    pub fn midias_mut(&mut self) -> Vec<&mut Midia> {
        match self {
            Bloco::Imagem { midia } | Bloco::Destaque { midia, .. } => vec![midia],
            Bloco::Cartoes { itens, .. } => {
                itens.iter_mut().filter_map(|i| i.midia.as_mut()).collect()
            }
            Bloco::Depoimentos { itens, .. } => {
                itens.iter_mut().filter_map(|i| i.foto.as_mut()).collect()
            }
            Bloco::Galeria { itens, .. } | Bloco::Logos { itens, .. } => itens.iter_mut().collect(),
            Bloco::Modelo3d { capa, .. } => capa.iter_mut().collect(),
            _ => Vec::new(),
        }
    }
}

/// Um campo de formulário.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Campo {
    /// O que o visitante lê acima do campo.
    pub rotulo: String,
    pub tipo: TipoDeCampo,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub obrigatorio: bool,
    /// As alternativas de um campo de escolha, na ordem em que aparecem.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opcoes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TipoDeCampo {
    /// Uma linha de texto.
    Texto,
    Email,
    Telefone,
    /// Texto longo, em várias linhas.
    Mensagem,
    /// Uma entre as alternativas de `opcoes`.
    Escolha,
}

impl TipoDeCampo {
    /// O máximo de caracteres que o campo aceita.
    pub fn limite(self) -> usize {
        match self {
            TipoDeCampo::Mensagem => 4000,
            TipoDeCampo::Texto
            | TipoDeCampo::Email
            | TipoDeCampo::Telefone
            | TipoDeCampo::Escolha => 200,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cartao {
    pub titulo: String,
    pub texto: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub midia: Option<Midia>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acao: Option<Acao>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Depoimento {
    pub texto: String,
    pub autor: String,
    /// Cargo, empresa ou cidade de quem falou.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foto: Option<Midia>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Numero {
    pub valor: String,
    pub rotulo: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Passo {
    pub titulo: String,
    pub texto: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plano {
    pub nome: String,
    /// Como aparece: "R$ 350" ou "Sob consulta".
    pub preco: String,
    /// "por mês", "por projeto".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub periodo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub descricao: Option<String>,
    /// O que o plano inclui.
    #[serde(default)]
    pub itens: Vec<String>,
    pub acao: Acao,
    /// O plano recomendado, que sai realçado.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub destaque: bool,
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
    /// O produto de catálogo, sem preço.
    Item(Item),
}

impl Documento {
    pub fn seo(&self) -> &Seo {
        match &self.conteudo {
            Conteudo::Pagina(p) => &p.seo,
            Conteudo::Post(p) => &p.seo,
            Conteudo::Produto(p) => &p.seo,
            Conteudo::Item(p) => &p.seo,
        }
    }

    /// O texto que vira o `<h1>`.
    pub fn titulo(&self) -> &str {
        match &self.conteudo {
            Conteudo::Pagina(p) => &p.titulo,
            Conteudo::Post(p) => &p.titulo,
            Conteudo::Produto(p) => &p.nome,
            Conteudo::Item(p) => &p.nome,
        }
    }

    pub fn slug(&self) -> &str {
        match &self.conteudo {
            Conteudo::Pagina(p) => &p.slug,
            Conteudo::Post(p) => &p.slug,
            Conteudo::Produto(p) => &p.slug,
            Conteudo::Item(p) => &p.slug,
        }
    }

    pub fn corpo(&self) -> &[Bloco] {
        match &self.conteudo {
            Conteudo::Pagina(p) => &p.corpo,
            Conteudo::Post(p) => &p.corpo,
            Conteudo::Produto(_) => &[],
            Conteudo::Item(p) => &p.corpo,
        }
    }

    pub fn trilha(&self) -> &[ItemTrilha] {
        match &self.conteudo {
            Conteudo::Pagina(p) => &p.trilha,
            Conteudo::Post(p) => &p.trilha,
            Conteudo::Produto(p) => &p.trilha,
            Conteudo::Item(p) => &p.trilha,
        }
    }

    pub fn atualizado_em(&self) -> DateTime<Utc> {
        match &self.conteudo {
            Conteudo::Pagina(p) => p.atualizado_em,
            Conteudo::Post(p) => p.atualizado_em,
            Conteudo::Produto(p) => p.atualizado_em,
            Conteudo::Item(p) => p.atualizado_em,
        }
    }

    /// A imagem principal: capa da página ou do post, primeira do produto
    /// (o da loja e o de catálogo).
    pub fn capa(&self) -> Option<&Midia> {
        match &self.conteudo {
            Conteudo::Pagina(p) => p.capa.as_ref(),
            Conteudo::Post(p) => Some(&p.capa),
            Conteudo::Produto(p) => p.imagens.first(),
            Conteudo::Item(p) => p.imagens.first(),
        }
    }

    /// Todas as imagens do documento, a principal primeiro.
    pub fn midias(&self) -> Vec<&Midia> {
        let mut midias: Vec<&Midia> = match &self.conteudo {
            Conteudo::Produto(p) => p.imagens.iter().collect(),
            Conteudo::Item(p) => p.imagens.iter().collect(),
            _ => self.capa().into_iter().collect(),
        };
        midias.extend(self.corpo().iter().flat_map(Bloco::midias));
        midias
    }

    /// Os modelos 3D do documento, na ordem em que aparecem: o do produto
    /// de catálogo e os do corpo.
    pub fn modelos_3d(&self) -> Vec<&Modelo3d> {
        let do_item = match &self.conteudo {
            Conteudo::Item(item) => item.modelo.as_ref(),
            _ => None,
        };
        do_item
            .into_iter()
            .chain(self.corpo().iter().filter_map(|bloco| match bloco {
                Bloco::Modelo3d { modelo, .. } => Some(modelo),
                _ => None,
            }))
            .collect()
    }

    /// A página precisa do visualizador 3D: é o único caso em que um site
    /// carrega script que não é a tag de medição.
    pub fn tem_modelo_3d(&self) -> bool {
        !self.modelos_3d().is_empty()
    }

    /// Página de apresentação: tem abertura ou alguma seção, e sai no
    /// leiaute largo.
    pub fn eh_larga(&self) -> bool {
        let com_abertura =
            matches!(&self.conteudo, Conteudo::Pagina(pagina) if pagina.abertura.is_some());
        com_abertura || self.corpo().iter().any(Bloco::eh_secao)
    }

    pub fn eh_home(&self) -> bool {
        matches!(&self.conteudo, Conteudo::Pagina(p) if p.slug.is_empty())
    }
}
