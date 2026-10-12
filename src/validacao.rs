//! As travas. Rascunho salva sempre; quem barra é a publicação.
//!
//! A validação devolve a lista de problemas, com os avisos junto. Não usa
//! `Result` porque o `Ok` esconderia os avisos justamente quando a página
//! pode ser publicada.

use std::fmt;

use serde::Serialize;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;
use unicode_segmentation::UnicodeSegmentation;

use crate::secoes::email_valido;
use crate::tipos::{Abertura, Acao, Bloco, Conteudo, Documento, Midia, Modelo3d, TipoDeCampo};
use crate::url::{incorporacao_de_video, link_valido};

/// O nome de uma página no menu: cabe em uma linha no celular.
pub const MENU_MAXIMO: usize = 24;
pub const TITULO_RECOMENDADO: usize = 60;
pub const TITULO_MAXIMO: usize = 70;
pub const DESCRICAO_RECOMENDADA: usize = 155;
pub const DESCRICAO_MAXIMA: usize = 170;
pub const LARGURA_MINIMA_DA_CAPA: u32 = 1200;
pub const UPLOAD_MAXIMO_BYTES: usize = 15 * 1024 * 1024;
/// Onde o site serve os modelos 3D da biblioteca dele.
pub const CAMINHO_DOS_MODELOS: &str = "/midia/modelos/";
/// O teto de um arquivo de modelo 3D (GLB ou USDZ). Modelo maior que isto
/// demora demais para abrir no celular.
pub const MODELO_MAXIMO_BYTES: usize = 30 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Gravidade {
    /// Impede publicar.
    Bloqueia,
    /// Aparece para quem edita, sem impedir.
    Avisa,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problema {
    /// Estável: é por ele que painel, API e conector reconhecem a regra.
    pub codigo: &'static str,
    /// Caminho do campo, como `corpo[3].midia.alt`.
    pub campo: String,
    pub gravidade: Gravidade,
    /// Para leigo: diz o que fazer.
    pub mensagem: String,
}

impl Problema {
    fn bloqueia(
        codigo: &'static str,
        campo: impl Into<String>,
        mensagem: impl Into<String>,
    ) -> Self {
        Self {
            codigo,
            campo: campo.into(),
            gravidade: Gravidade::Bloqueia,
            mensagem: mensagem.into(),
        }
    }

    fn avisa(codigo: &'static str, campo: impl Into<String>, mensagem: impl Into<String>) -> Self {
        Self {
            codigo,
            campo: campo.into(),
            gravidade: Gravidade::Avisa,
            mensagem: mensagem.into(),
        }
    }
}

/// O que a regra precisa saber do resto do site. Quem chama monta; o motor
/// não consulta nada.
#[derive(Debug, Clone, Copy)]
pub struct Contexto<'a> {
    /// Títulos de busca dos outros documentos do site.
    pub titulos_em_uso: &'a [String],
    pub descricoes_em_uso: &'a [String],
    /// Slug com que o documento está publicado hoje, se estiver.
    pub slug_publicado: Option<&'a str>,
    /// Slugs antigos que já têm redirecionamento.
    pub redirecionados: &'a [String],
    /// Peso medido da página, em kB.
    pub peso_kb: Option<u32>,
    pub orcamento_peso_kb: u32,
}

impl Contexto<'static> {
    /// Sem informação do resto do site: só as regras do próprio documento.
    pub fn vazio() -> Self {
        Self {
            titulos_em_uso: &[],
            descricoes_em_uso: &[],
            slug_publicado: None,
            redirecionados: &[],
            peso_kb: None,
            orcamento_peso_kb: u32::MAX,
        }
    }
}

pub fn pode_publicar(problemas: &[Problema]) -> bool {
    problemas.iter().all(|p| p.gravidade != Gravidade::Bloqueia)
}

/// Tira acento, baixa a caixa e troca o que não for letra ou número por
/// hífen, sem hífen repetido nem nas pontas.
pub fn normalizar_slug(texto: &str) -> String {
    let mut slug = String::with_capacity(texto.len());
    // Decomposição de compatibilidade: "ª" vira "a", "ﬁ" vira "fi".
    for caractere in texto.nfkd().filter(|c| !is_combining_mark(*c)) {
        if caractere.is_ascii_alphanumeric() {
            slug.push(caractere.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

/// Comprimento como a pessoa vê: "Ação" tem quatro.
pub fn comprimento(texto: &str) -> usize {
    texto.nfc().collect::<String>().graphemes(true).count()
}

/// "Atualizado em" só muda quando o que o visitante lê mudou.
pub fn deve_atualizar_data(antes: &Documento, depois: &Documento) -> bool {
    match (&antes.conteudo, &depois.conteudo) {
        (Conteudo::Pagina(a), Conteudo::Pagina(d)) => {
            a.titulo != d.titulo || a.corpo != d.corpo || a.capa != d.capa
        }
        (Conteudo::Post(a), Conteudo::Post(d)) => {
            a.titulo != d.titulo || a.resumo != d.resumo || a.corpo != d.corpo || a.capa != d.capa
        }
        (Conteudo::Item(a), Conteudo::Item(d)) => {
            a.nome != d.nome
                || a.resumo != d.resumo
                || a.imagens != d.imagens
                || a.modelo != d.modelo
                || a.ficha != d.ficha
                || a.corpo != d.corpo
        }
        (Conteudo::Produto(a), Conteudo::Produto(d)) => {
            a.nome != d.nome
                || a.descricao != d.descricao
                || a.imagens != d.imagens
                || a.variacoes != d.variacoes
        }
        _ => true,
    }
}

pub fn validar(documento: &Documento, contexto: &Contexto) -> Vec<Problema> {
    let mut problemas = Vec::new();

    validar_busca(documento, contexto, &mut problemas);
    validar_slug(documento, contexto, &mut problemas);
    if documento.titulo().trim().is_empty() {
        let campo = if matches!(documento.conteudo, Conteudo::Produto(_) | Conteudo::Item(_)) {
            "nome"
        } else {
            "titulo"
        };
        problemas.push(Problema::bloqueia(
            "titulo.vazio",
            campo,
            "Dê um título. É ele que aparece no topo da página.",
        ));
    }
    validar_corpo(documento.corpo(), &mut problemas);

    match &documento.conteudo {
        Conteudo::Pagina(pagina) => {
            if let Some(capa) = &pagina.capa {
                validar_alt(capa, "capa.alt", &mut problemas);
            }
            if let Some(abertura) = &pagina.abertura {
                validar_abertura(abertura, &mut problemas);
            }
            if pagina
                .menu
                .as_deref()
                .is_some_and(|menu| comprimento(menu.trim()) > MENU_MAXIMO)
            {
                problemas.push(Problema::bloqueia(
                    "menu.longo",
                    "menu",
                    format!(
                        "O nome no menu passa de {MENU_MAXIMO} caracteres. Use uma ou duas palavras."
                    ),
                ));
            }
        }
        Conteudo::Post(post) => {
            validar_alt(&post.capa, "capa.alt", &mut problemas);
            if post.capa.largura < LARGURA_MINIMA_DA_CAPA {
                problemas.push(Problema::bloqueia(
                    "post.capa.pequena",
                    "capa",
                    format!(
                        "A imagem de capa tem {} px de largura. Envie uma com pelo menos {LARGURA_MINIMA_DA_CAPA} px para ela aparecer em destaque no Google.",
                        post.capa.largura
                    ),
                ));
            }
            if post.autor.bio.trim().is_empty() {
                problemas.push(Problema::bloqueia(
                    "post.autor.incompleto",
                    "autor.bio",
                    "Escreva a apresentação do autor. Quem lê quer saber quem assina o texto.",
                ));
            }
            if post.autor.foto.id.trim().is_empty() {
                problemas.push(Problema::bloqueia(
                    "post.autor.incompleto",
                    "autor.foto",
                    "Envie a foto do autor.",
                ));
            } else {
                validar_alt(&post.autor.foto, "autor.foto.alt", &mut problemas);
            }
        }
        Conteudo::Item(item) => {
            if item.resumo.trim().is_empty() {
                problemas.push(Problema::bloqueia(
                    "item.resumo.vazio",
                    "resumo",
                    "Escreva uma ou duas frases sobre o produto. Elas aparecem no topo da página e na listagem.",
                ));
            }
            for (i, imagem) in item.imagens.iter().enumerate() {
                validar_alt(imagem, &format!("imagens[{i}].alt"), &mut problemas);
            }
            if let Some(modelo) = &item.modelo {
                validar_modelo_3d(modelo, "modelo", &mut problemas);
            }
            for (i, linha) in item.ficha.iter().enumerate() {
                if linha.nome.trim().is_empty() || linha.valor.trim().is_empty() {
                    problemas.push(Problema::bloqueia(
                        "item.ficha.incompleta",
                        format!("ficha[{i}]"),
                        "Cada linha da ficha técnica precisa do nome e do valor, como Peso e 156 kg.",
                    ));
                }
            }
            if let Some(acao) = &item.acao {
                validar_acao(acao, "acao", &mut problemas);
            }
        }
        Conteudo::Produto(produto) => {
            for (i, imagem) in produto.imagens.iter().enumerate() {
                validar_alt(imagem, &format!("imagens[{i}].alt"), &mut problemas);
            }
            if produto.variacoes.is_empty() {
                problemas.push(Problema::bloqueia(
                    "produto.sem-variacao",
                    "variacoes",
                    "Cadastre ao menos uma opção de venda, com preço.",
                ));
            }
            for (i, variacao) in produto.variacoes.iter().enumerate() {
                if variacao.preco_centavos <= 0 {
                    problemas.push(Problema::bloqueia(
                        "produto.preco",
                        format!("variacoes[{i}].precoCentavos"),
                        "Informe um preço maior que zero.",
                    ));
                }
            }
        }
    }

    if !documento.seo().indexar {
        problemas.push(Problema::avisa(
            "seo.noindex",
            "seo.indexar",
            "Esta página está escondida do Google e de outros buscadores. Ninguém vai encontrá-la pela busca.",
        ));
    }
    if let Some(peso) = contexto
        .peso_kb
        .filter(|peso| *peso > contexto.orcamento_peso_kb)
    {
        problemas.push(Problema::avisa(
            "pagina.peso",
            "",
            format!(
                "A página pesa {peso} kB e o recomendado é até {} kB. Páginas pesadas demoram a abrir no celular.",
                contexto.orcamento_peso_kb
            ),
        ));
    }

    problemas
}

fn validar_busca(documento: &Documento, contexto: &Contexto, problemas: &mut Vec<Problema>) {
    let seo = documento.seo();
    let igual = |a: &str, b: &str| a.trim().to_lowercase() == b.trim().to_lowercase();

    let titulo = seo.titulo.trim();
    if titulo.is_empty() {
        problemas.push(Problema::bloqueia(
            "seo.titulo.vazio",
            "seo.titulo",
            "Escreva o título que vai aparecer no Google.",
        ));
    } else {
        if contexto
            .titulos_em_uso
            .iter()
            .any(|outro| igual(outro, titulo))
        {
            problemas.push(Problema::bloqueia(
                "seo.titulo.repetido",
                "seo.titulo",
                "Outra página do site já usa este título. Cada página precisa do seu, para o Google saber qual mostrar.",
            ));
        }
        let tamanho = comprimento(titulo);
        if tamanho > TITULO_MAXIMO {
            problemas.push(Problema::bloqueia(
                "seo.titulo.excedido",
                "seo.titulo",
                format!(
                    "O título tem {tamanho} caracteres. Reduza para no máximo {TITULO_MAXIMO}."
                ),
            ));
        } else if tamanho > TITULO_RECOMENDADO {
            problemas.push(Problema::avisa(
                "seo.titulo.longo",
                "seo.titulo",
                format!("O título tem {tamanho} caracteres. Acima de {TITULO_RECOMENDADO} o Google costuma cortar o final."),
            ));
        }
    }

    let descricao = seo.descricao.trim();
    if descricao.is_empty() {
        problemas.push(Problema::bloqueia(
            "seo.descricao.vazia",
            "seo.descricao",
            "Escreva a descrição que aparece embaixo do título no Google.",
        ));
    } else {
        if contexto
            .descricoes_em_uso
            .iter()
            .any(|outra| igual(outra, descricao))
        {
            problemas.push(Problema::bloqueia(
                "seo.descricao.repetida",
                "seo.descricao",
                "Outra página do site já usa esta descrição. Escreva uma própria para esta página.",
            ));
        }
        let tamanho = comprimento(descricao);
        if tamanho > DESCRICAO_MAXIMA {
            problemas.push(Problema::bloqueia(
                "seo.descricao.excedida",
                "seo.descricao",
                format!("A descrição tem {tamanho} caracteres. Reduza para no máximo {DESCRICAO_MAXIMA}."),
            ));
        } else if tamanho > DESCRICAO_RECOMENDADA {
            problemas.push(Problema::avisa(
                "seo.descricao.longa",
                "seo.descricao",
                format!(
                    "A descrição tem {tamanho} caracteres. Acima de {DESCRICAO_RECOMENDADA} o Google costuma cortar o final."
                ),
            ));
        }
    }
}

fn validar_slug(documento: &Documento, contexto: &Contexto, problemas: &mut Vec<Problema>) {
    let slug = documento.slug();
    let eh_pagina = matches!(documento.conteudo, Conteudo::Pagina(_));

    if slug.is_empty() {
        // Só a home, que é uma página, pode não ter endereço próprio.
        if !eh_pagina {
            problemas.push(Problema::bloqueia(
                "slug.vazio",
                "slug",
                "Defina o endereço desta página.",
            ));
        }
    } else if normalizar_slug(slug).is_empty() {
        problemas.push(Problema::bloqueia(
            "slug.vazio",
            "slug",
            "O endereço precisa ter letras ou números.",
        ));
    } else if normalizar_slug(slug) != slug {
        problemas.push(Problema::bloqueia(
            "slug.formato",
            "slug",
            format!(
                "O endereço só pode ter letras minúsculas, números e hífen. Use \"{}\".",
                normalizar_slug(slug)
            ),
        ));
    }

    if let Some(anterior) = contexto.slug_publicado {
        if anterior != slug && !contexto.redirecionados.iter().any(|r| r == anterior) {
            problemas.push(Problema::bloqueia(
                "slug.sem-redirecionamento",
                "slug",
                "Esta página já está no ar com outro endereço. Crie o redirecionamento do endereço antigo para quem tem o link não cair em página inexistente.",
            ));
        }
    }
}

fn validar_corpo(blocos: &[Bloco], problemas: &mut Vec<Problema>) {
    // O título da página é o nível 1; o corpo começa no 2.
    let mut nivel_anterior = 1u8;
    for (i, bloco) in blocos.iter().enumerate() {
        match bloco {
            Bloco::Titulo { nivel, .. } => {
                if !(2..=4).contains(nivel) || *nivel > nivel_anterior + 1 {
                    problemas.push(Problema::bloqueia(
                        "corpo.titulo.nivel",
                        format!("corpo[{i}].nivel"),
                        format!(
                            "Este título pula um nível. Depois de um título de nível {nivel_anterior} vem um de nível {}, no máximo.",
                            nivel_anterior + 1
                        ),
                    ));
                }
                nivel_anterior = (*nivel).clamp(2, 4);
            }
            Bloco::Imagem { midia } => {
                validar_alt(midia, &format!("corpo[{i}].midia.alt"), problemas)
            }
            Bloco::Paragrafo { trechos } => {
                for (j, trecho) in trechos.iter().enumerate() {
                    if trecho
                        .link
                        .as_deref()
                        .is_some_and(|link| !link_valido(link))
                    {
                        problemas.push(link_invalido(format!("corpo[{i}].trechos[{j}].link")));
                    }
                }
            }
            Bloco::Chamada { url, .. } => {
                if !link_valido(url) {
                    problemas.push(link_invalido(format!("corpo[{i}].url")));
                }
            }
            Bloco::Modelo3d { modelo, capa, .. } => {
                validar_modelo_3d(modelo, &format!("corpo[{i}].modelo"), problemas);
                if let Some(capa) = capa {
                    validar_alt(capa, &format!("corpo[{i}].capa.alt"), problemas);
                }
            }
            Bloco::Video { url, .. } => {
                if incorporacao_de_video(url).is_none() {
                    problemas.push(Problema::bloqueia(
                        "video.origem",
                        format!("corpo[{i}].url"),
                        "Use o endereço de um vídeo do YouTube ou do Vimeo.",
                    ));
                }
            }
            Bloco::Lista { .. }
            | Bloco::Citacao { .. }
            | Bloco::Tabela { .. }
            | Bloco::Perguntas { .. } => {}
            secao => {
                // O título da seção é de nível 2, e o de cada item, 3.
                if let Some(nivel) = validar_secao(secao, &format!("corpo[{i}]"), problemas) {
                    nivel_anterior = nivel;
                }
            }
        }
    }
}

fn texto_faltando(campo: String) -> Problema {
    Problema::bloqueia(
        "secao.texto.vazio",
        campo,
        "Falta preencher um texto desta seção.",
    )
}

fn validar_acao(acao: &Acao, campo: &str, problemas: &mut Vec<Problema>) {
    if acao.rotulo.trim().is_empty() {
        problemas.push(texto_faltando(format!("{campo}.rotulo")));
    }
    if !link_valido(&acao.url) {
        problemas.push(link_invalido(format!("{campo}.url")));
    }
}

/// Imagem obrigatória de uma seção: tem de ter sido escolhida e descrita.
fn validar_imagem_da_secao(midia: &Midia, campo: &str, problemas: &mut Vec<Problema>) {
    if midia.id.trim().is_empty() {
        problemas.push(Problema::bloqueia(
            "secao.imagem.falta",
            campo,
            "Escolha a imagem desta seção.",
        ));
    } else {
        validar_alt(midia, &format!("{campo}.alt"), problemas);
    }
}

/// Confere uma seção e devolve o nível do último título que ela gera, ou
/// `None` quando o bloco não é seção ou não gera título.
fn validar_secao(bloco: &Bloco, campo: &str, problemas: &mut Vec<Problema>) -> Option<u8> {
    let preenchido =
        |texto: &Option<String>| texto.as_deref().is_some_and(|t| !t.trim().is_empty());
    let exigir = |texto: &str, nome: String, problemas: &mut Vec<Problema>| {
        if texto.trim().is_empty() {
            problemas.push(texto_faltando(format!("{campo}.{nome}")));
        }
    };
    // Sem item, a seção sairia como uma faixa vazia.
    let (tem_titulo, itens, titulo_nos_itens) = match bloco {
        Bloco::Cartoes { titulo, itens, .. } => {
            for (j, item) in itens.iter().enumerate() {
                exigir(&item.titulo, format!("itens[{j}].titulo"), problemas);
                exigir(&item.texto, format!("itens[{j}].texto"), problemas);
                if let Some(midia) = &item.midia {
                    validar_imagem_da_secao(midia, &format!("{campo}.itens[{j}].midia"), problemas);
                }
                if let Some(acao) = &item.acao {
                    validar_acao(acao, &format!("{campo}.itens[{j}].acao"), problemas);
                }
            }
            (preenchido(titulo), itens.len(), true)
        }
        Bloco::Destaque {
            titulo,
            texto,
            midia,
            acao,
            ..
        } => {
            exigir(titulo, "titulo".into(), problemas);
            exigir(texto, "texto".into(), problemas);
            validar_imagem_da_secao(midia, &format!("{campo}.midia"), problemas);
            if let Some(acao) = acao {
                validar_acao(acao, &format!("{campo}.acao"), problemas);
            }
            (true, 1, false)
        }
        Bloco::Depoimentos { titulo, itens } => {
            for (j, item) in itens.iter().enumerate() {
                exigir(&item.texto, format!("itens[{j}].texto"), problemas);
                exigir(&item.autor, format!("itens[{j}].autor"), problemas);
                if let Some(foto) = &item.foto {
                    validar_imagem_da_secao(foto, &format!("{campo}.itens[{j}].foto"), problemas);
                }
            }
            (preenchido(titulo), itens.len(), false)
        }
        Bloco::Numeros { titulo, itens } => {
            for (j, item) in itens.iter().enumerate() {
                exigir(&item.valor, format!("itens[{j}].valor"), problemas);
                exigir(&item.rotulo, format!("itens[{j}].rotulo"), problemas);
            }
            (preenchido(titulo), itens.len(), false)
        }
        Bloco::Passos { titulo, itens } => {
            for (j, item) in itens.iter().enumerate() {
                exigir(&item.titulo, format!("itens[{j}].titulo"), problemas);
                exigir(&item.texto, format!("itens[{j}].texto"), problemas);
            }
            (preenchido(titulo), itens.len(), true)
        }
        Bloco::Planos { titulo, itens } => {
            for (j, plano) in itens.iter().enumerate() {
                exigir(&plano.nome, format!("itens[{j}].nome"), problemas);
                exigir(&plano.preco, format!("itens[{j}].preco"), problemas);
                validar_acao(&plano.acao, &format!("{campo}.itens[{j}].acao"), problemas);
            }
            (preenchido(titulo), itens.len(), true)
        }
        Bloco::Galeria { titulo, itens } | Bloco::Logos { titulo, itens } => {
            for (j, midia) in itens.iter().enumerate() {
                validar_imagem_da_secao(midia, &format!("{campo}.itens[{j}]"), problemas);
            }
            (preenchido(titulo), itens.len(), false)
        }
        Bloco::Faixa { titulo, acoes, .. } => {
            exigir(titulo, "titulo".into(), problemas);
            for (j, acao) in acoes.iter().enumerate() {
                validar_acao(acao, &format!("{campo}.acoes[{j}]"), problemas);
            }
            if acoes.len() > MAXIMO_DE_BOTOES {
                problemas.push(botoes_demais(format!("{campo}.acoes")));
            }
            (true, acoes.len(), false)
        }
        Bloco::Formulario {
            titulo,
            campos,
            botao,
            agradecimento,
            ..
        } => {
            for (j, item) in campos.iter().enumerate() {
                exigir(&item.rotulo, format!("campos[{j}].rotulo"), problemas);
                let alternativas = item
                    .opcoes
                    .iter()
                    .filter(|opcao| !opcao.trim().is_empty())
                    .count();
                if item.tipo == TipoDeCampo::Escolha && alternativas < 2 {
                    problemas.push(Problema::bloqueia(
                        "formulario.opcoes",
                        format!("{campo}.campos[{j}].opcoes"),
                        "Campo de escolha precisa de pelo menos duas alternativas.",
                    ));
                }
            }
            exigir(botao, "botao".into(), problemas);
            exigir(agradecimento, "agradecimento".into(), problemas);
            if campos.len() > MAXIMO_DE_CAMPOS {
                problemas.push(Problema::bloqueia(
                    "formulario.campos",
                    format!("{campo}.campos"),
                    format!(
                        "O formulário tem campos demais. Peça só o necessário: até {MAXIMO_DE_CAMPOS} campos."
                    ),
                ));
            }
            (preenchido(titulo), campos.len(), false)
        }
        Bloco::Contato {
            titulo,
            telefone,
            whatsapp,
            email,
            endereco,
            ..
        } => {
            if email
                .as_deref()
                .is_some_and(|e| !e.trim().is_empty() && !email_valido(e))
            {
                problemas.push(Problema::bloqueia(
                    "contato.email",
                    format!("{campo}.email"),
                    "O e-mail de contato não parece um endereço de e-mail.",
                ));
            }
            let canais = [telefone, whatsapp, email, endereco]
                .into_iter()
                .filter(|canal| preenchido(canal))
                .count();
            (preenchido(titulo), canais, false)
        }
        _ => return None,
    };
    if itens == 0 {
        problemas.push(Problema::bloqueia(
            "secao.vazia",
            format!("{campo}.itens"),
            "Esta seção está vazia. Preencha pelo menos um item ou remova a seção.",
        ));
    }
    match (tem_titulo, titulo_nos_itens && itens > 0) {
        (true, true) => Some(3),
        (true, false) => Some(2),
        // Sem título de seção, o dos itens sairia solto no nível 3.
        (false, true) => {
            problemas.push(Problema::bloqueia(
                "secao.titulo.vazio",
                format!("{campo}.titulo"),
                "Dê um título a esta seção: os itens dela têm título, e ele precisa vir abaixo de um.",
            ));
            None
        }
        (false, false) => None,
    }
}

/// Formulário longo afasta quem ia escrever, e cada campo é dado a guardar.
pub const MAXIMO_DE_CAMPOS: usize = 8;

/// Mais de dois botões lado a lado, e nenhum é o principal.
const MAXIMO_DE_BOTOES: usize = 2;

fn botoes_demais(campo: String) -> Problema {
    Problema::bloqueia(
        "secao.botoes",
        campo,
        "Use no máximo dois botões: o principal e um secundário.",
    )
}

/// A abertura de uma página de apresentação.
fn validar_abertura(abertura: &Abertura, problemas: &mut Vec<Problema>) {
    if abertura.texto.trim().is_empty() {
        problemas.push(texto_faltando("abertura.texto".into()));
    }
    for (j, acao) in abertura.acoes.iter().enumerate() {
        validar_acao(acao, &format!("abertura.acoes[{j}]"), problemas);
    }
    if abertura.acoes.len() > MAXIMO_DE_BOTOES {
        problemas.push(botoes_demais("abertura.acoes".into()));
    }
}

fn validar_modelo_3d(modelo: &Modelo3d, campo: &str, problemas: &mut Vec<Problema>) {
    if modelo.descricao.trim().is_empty() {
        problemas.push(Problema::bloqueia(
            "modelo.descricao.vazia",
            format!("{campo}.descricao"),
            "Descreva o que o modelo 3D mostra, para quem não o enxerga.",
        ));
    }
    // Só arquivo da biblioteca do próprio site: o visualizador não abre
    // modelo de outro endereço.
    let da_biblioteca = |endereco: &str, extensao: &str| {
        endereco.starts_with(CAMINHO_DOS_MODELOS) && endereco.ends_with(extensao)
    };
    let arquivos_certos = da_biblioteca(&modelo.glb, ".glb")
        && modelo
            .usdz
            .as_deref()
            .is_none_or(|usdz| da_biblioteca(usdz, ".usdz"));
    if !arquivos_certos {
        problemas.push(Problema::bloqueia(
            "modelo.arquivo",
            campo,
            "Escolha um modelo 3D da biblioteca do site.",
        ));
    }
}

fn link_invalido(campo: String) -> Problema {
    Problema::bloqueia(
        "link.invalido",
        campo,
        "Este link não é válido. Use um endereço que comece com https:// ou um caminho do próprio site, como /contato.",
    )
}

fn validar_alt(midia: &Midia, campo: &str, problemas: &mut Vec<Problema>) {
    validar_direitos(midia, campo.trim_end_matches(".alt"), problemas);
    let alt = midia.alt.trim();
    if alt.is_empty() {
        problemas.push(Problema::bloqueia(
            "midia.alt.vazio",
            campo,
            "Escreva uma descrição da foto para quem não enxerga a imagem.",
        ));
    } else if parece_nome_de_arquivo(alt, midia) {
        problemas.push(Problema::bloqueia(
            "midia.alt.arquivo",
            campo,
            "A descrição da foto é o nome do arquivo. Descreva o que aparece na imagem.",
        ));
    }
}

/// Licença e página de aquisição são endereços: saem no dado estruturado da
/// imagem, e endereço quebrado ali tira o selo de imagem licenciável.
fn validar_direitos(midia: &Midia, campo: &str, problemas: &mut Vec<Problema>) {
    let direitos = &midia.direitos;
    for (nome, endereco) in [
        ("licenca", &direitos.licenca),
        ("aquisicao", &direitos.aquisicao),
    ] {
        if endereco.as_deref().is_some_and(|e| !link_valido(e.trim())) {
            problemas.push(Problema::bloqueia(
                "midia.direitos.endereco",
                format!("{campo}.direitos.{nome}"),
                "O endereço da licença da imagem não é válido. Use um que comece com https:// ou um caminho do próprio site.",
            ));
        }
    }
}

fn parece_nome_de_arquivo(alt: &str, midia: &Midia) -> bool {
    let minusculo = alt.to_lowercase();
    if [".jpg", ".jpeg", ".png", ".webp", ".avif", ".gif"]
        .iter()
        .any(|extensao| minusculo.ends_with(extensao))
    {
        return true;
    }
    let alt = normalizar_slug(alt);
    midia.variantes.iter().any(|variante| {
        let nome = nome_base(&variante.url);
        !nome.is_empty() && nome == alt
    })
}

/// O nome original do arquivo, deduzido da URL da variante: sem pasta, sem
/// extensão, sem largura e sem o trecho do hash.
fn nome_base(url: &str) -> String {
    let arquivo = url
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("");
    let sem_extensao = arquivo.rsplit_once('.').map_or(arquivo, |(nome, _)| nome);
    let mut partes: Vec<&str> = sem_extensao.split('-').collect();
    if partes
        .last()
        .is_some_and(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    {
        partes.pop();
    }
    if partes
        .last()
        .is_some_and(|p| p.len() == 8 && p.chars().all(|c| c.is_ascii_hexdigit()))
    {
        partes.pop();
    }
    normalizar_slug(&partes.join("-"))
}

/// Formatos de arquivo aceitos no envio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatoDeEnvio {
    Jpeg,
    Png,
    Webp,
}

/// Lê o formato dos primeiros bytes, não da extensão.
pub fn formato_do_arquivo(bytes: &[u8]) -> Option<FormatoDeEnvio> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(FormatoDeEnvio::Jpeg)
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some(FormatoDeEnvio::Png)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(FormatoDeEnvio::Webp)
    } else {
        None
    }
}

pub fn validar_upload(nome: &str, bytes: &[u8]) -> Vec<Problema> {
    let mut problemas = Vec::new();
    if bytes.len() > UPLOAD_MAXIMO_BYTES {
        problemas.push(Problema::bloqueia(
            "upload.grande",
            nome,
            format!(
                "O arquivo tem {} MB. O limite é {} MB.",
                bytes.len().div_ceil(1024 * 1024),
                UPLOAD_MAXIMO_BYTES / (1024 * 1024)
            ),
        ));
    }
    if formato_do_arquivo(bytes).is_none() {
        problemas.push(Problema::bloqueia(
            "upload.formato",
            nome,
            "Envie a imagem em JPG, PNG ou WebP.",
        ));
    }
    problemas
}

/// O documento não passaria na validação sem contexto. É falha de quem
/// chamou o gerador, não de quem editou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentoInvalido {
    pub caminho: String,
    pub codigo: &'static str,
    pub campo: String,
}

impl fmt::Display for DocumentoInvalido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "documento {} não pode ser publicado: {} em \"{}\"",
            self.caminho, self.codigo, self.campo
        )
    }
}

impl std::error::Error for DocumentoInvalido {}

pub(crate) fn exigir_publicavel(documento: &Documento) -> Result<(), DocumentoInvalido> {
    match validar(documento, &Contexto::vazio())
        .into_iter()
        .find(|p| p.gravidade == Gravidade::Bloqueia)
    {
        Some(problema) => Err(DocumentoInvalido {
            caminho: documento.caminho.clone(),
            codigo: problema.codigo,
            campo: problema.campo,
        }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::demonstracao;
    use crate::tipos::{Trecho, Variacao};

    fn codigos(problemas: &[Problema]) -> Vec<&'static str> {
        problemas.iter().map(|p| p.codigo).collect()
    }

    fn validar_sem_contexto(documento: &Documento) -> Vec<Problema> {
        validar(documento, &Contexto::vazio())
    }

    fn com_post(altera: impl FnOnce(&mut crate::tipos::Post)) -> Documento {
        let mut documento = demonstracao::post_de_exemplo();
        if let Conteudo::Post(post) = &mut documento.conteudo {
            altera(post);
        }
        documento
    }

    fn com_produto(altera: impl FnOnce(&mut crate::tipos::Product)) -> Documento {
        let mut documento = demonstracao::produto_de_exemplo();
        if let Conteudo::Produto(produto) = &mut documento.conteudo {
            altera(produto);
        }
        documento
    }

    #[test]
    fn exemplos_passam_limpos() {
        assert_eq!(
            validar_sem_contexto(&demonstracao::post_de_exemplo()),
            vec![]
        );
        assert_eq!(
            validar_sem_contexto(&demonstracao::produto_de_exemplo()),
            vec![]
        );
    }

    fn com_apresentacao(altera: impl FnOnce(&mut crate::tipos::Pagina)) -> Vec<Problema> {
        let mut documento = demonstracao::apresentacao();
        if let Conteudo::Pagina(pagina) = &mut documento.conteudo {
            altera(pagina);
        }
        validar_sem_contexto(&documento)
    }

    #[test]
    fn pagina_de_apresentacao_passa_limpa() {
        assert_eq!(com_apresentacao(|_| {}), vec![]);
    }

    #[test]
    fn secao_vazia_ou_incompleta_nao_vai_ao_ar() {
        use crate::tipos::{Acao, Cartao, Midia};

        let problemas = com_apresentacao(|pagina| {
            pagina.corpo = vec![
                Bloco::Numeros {
                    titulo: None,
                    itens: vec![],
                },
                Bloco::Cartoes {
                    titulo: None,
                    texto: None,
                    itens: vec![Cartao {
                        titulo: "Restauro".into(),
                        texto: " ".into(),
                        midia: None,
                        acao: Some(Acao {
                            rotulo: "Ver".into(),
                            url: "javascript:alert(1)".into(),
                        }),
                    }],
                },
                Bloco::Destaque {
                    titulo: "Madeira".into(),
                    texto: "Com procedência.".into(),
                    midia: Midia {
                        id: String::new(),
                        alt: String::new(),
                        largura: 0,
                        altura: 0,
                        variantes: vec![],
                        legenda: None,
                        credito: None,
                        direitos: Default::default(),
                    },
                    invertido: false,
                    acao: None,
                },
                Bloco::Contato {
                    titulo: None,
                    texto: None,
                    telefone: None,
                    whatsapp: None,
                    email: Some("sem-arroba".into()),
                    endereco: None,
                    horario: Some("Das 8h às 18h".into()),
                },
            ];
        });
        let achados: Vec<(&str, &str)> = problemas
            .iter()
            .map(|p| (p.codigo, p.campo.as_str()))
            .collect();
        assert_eq!(
            achados,
            vec![
                ("secao.vazia", "corpo[0].itens"),
                ("secao.texto.vazio", "corpo[1].itens[0].texto"),
                ("link.invalido", "corpo[1].itens[0].acao.url"),
                ("secao.titulo.vazio", "corpo[1].titulo"),
                ("secao.imagem.falta", "corpo[2].midia"),
                ("contato.email", "corpo[3].email"),
            ]
        );
    }

    #[test]
    fn abertura_pede_texto_e_no_maximo_dois_botoes() {
        use crate::tipos::Acao;

        let problemas = com_apresentacao(|pagina| {
            let abertura = pagina.abertura.as_mut().expect("abertura");
            abertura.texto = String::new();
            abertura.acoes.push(Acao {
                rotulo: "Terceiro".into(),
                url: "/sobre".into(),
            });
        });
        assert_eq!(
            codigos(&problemas),
            vec!["secao.texto.vazio", "secao.botoes"]
        );
    }

    #[test]
    fn formulario_pede_campo_botao_e_agradecimento() {
        use crate::tipos::{Campo, TipoDeCampo};

        let problemas = com_apresentacao(|pagina| {
            pagina.corpo = vec![
                Bloco::Formulario {
                    titulo: Some("Contato".into()),
                    texto: None,
                    campos: vec![],
                    botao: " ".into(),
                    agradecimento: "Obrigado.".into(),
                },
                Bloco::Formulario {
                    titulo: None,
                    texto: None,
                    campos: (0..9)
                        .map(|i| Campo {
                            rotulo: format!("Campo {i}"),
                            tipo: TipoDeCampo::Texto,
                            obrigatorio: false,
                            opcoes: vec![],
                        })
                        .collect(),
                    botao: "Enviar".into(),
                    agradecimento: "Obrigado.".into(),
                },
            ];
        });
        let achados: Vec<(&str, &str)> = problemas
            .iter()
            .map(|p| (p.codigo, p.campo.as_str()))
            .collect();
        assert_eq!(
            achados,
            vec![
                ("secao.texto.vazio", "corpo[0].botao"),
                ("secao.vazia", "corpo[0].itens"),
                ("formulario.campos", "corpo[1].campos"),
            ]
        );
    }

    #[test]
    fn campo_de_escolha_precisa_de_alternativas() {
        use crate::tipos::{Campo, TipoDeCampo};

        let com = |opcoes: &[&str]| {
            com_apresentacao(|pagina| {
                pagina.corpo = vec![Bloco::Formulario {
                    titulo: None,
                    texto: None,
                    campos: vec![Campo {
                        rotulo: "Momento".into(),
                        tipo: TipoDeCampo::Escolha,
                        obrigatorio: true,
                        opcoes: opcoes.iter().map(|o| (*o).to_string()).collect(),
                    }],
                    botao: "Enviar".into(),
                    agradecimento: "Obrigado.".into(),
                }];
            })
        };
        assert_eq!(
            codigos(&com(&["Começando", " "])),
            vec!["formulario.opcoes"]
        );
        assert_eq!(com(&["Começando", "Crescendo"]), vec![]);
    }

    #[test]
    fn nome_no_menu_e_curto() {
        let problemas = com_apresentacao(|pagina| {
            pagina.menu = Some("Móveis de madeira maciça feitos para durar".into());
        });
        assert_eq!(codigos(&problemas), vec!["menu.longo"]);
    }

    #[test]
    fn titulo_de_texto_pode_vir_depois_de_uma_secao() {
        let problemas = com_apresentacao(|pagina| {
            pagina.corpo.push(Bloco::Titulo {
                nivel: 2,
                texto: "Mais sobre a oficina".into(),
            });
        });
        assert_eq!(problemas, vec![]);
    }

    #[test]
    fn licenca_de_imagem_precisa_ser_endereco() {
        let documento = com_post(|post| post.capa.direitos.licenca = Some("cc by 4.0".into()));
        let problemas = validar_sem_contexto(&documento);
        assert_eq!(codigos(&problemas), vec!["midia.direitos.endereco"]);
        assert_eq!(problemas[0].campo, "capa.direitos.licenca");

        let documento = com_post(|post| {
            post.capa.direitos.licenca = Some("/licenca-de-imagens".into());
            post.capa.direitos.aquisicao = Some("https://exemplo.example/licenca".into());
        });
        assert_eq!(validar_sem_contexto(&documento), vec![]);
    }

    #[test]
    fn slug_perde_acento_caixa_e_sobra() {
        assert_eq!(
            normalizar_slug("  Ação & Reação: 2ª edição!  "),
            "acao-reacao-2a-edicao"
        );
        assert_eq!(
            normalizar_slug("São_João--da__Boa Vista"),
            "sao-joao-da-boa-vista"
        );
        assert_eq!(normalizar_slug("!!!"), "");
    }

    #[test]
    fn comprimento_conta_o_que_a_pessoa_ve() {
        assert_eq!(comprimento("Ação"), 4);
        // "a" seguido de til combinante é uma letra só.
        assert_eq!(comprimento("a\u{0303}o"), 2);
        assert_eq!(comprimento("👨‍👩‍👧 família"), 9);
    }

    #[test]
    fn titulo_vazio_longo_e_excedido() {
        let vazio = com_post(|p| p.seo.titulo = "  ".into());
        assert!(codigos(&validar_sem_contexto(&vazio)).contains(&"seo.titulo.vazio"));

        let no_limite = com_post(|p| p.seo.titulo = "ã".repeat(60));
        assert_eq!(validar_sem_contexto(&no_limite), vec![]);

        let longo = com_post(|p| p.seo.titulo = "ã".repeat(61));
        let problemas = validar_sem_contexto(&longo);
        assert_eq!(codigos(&problemas), vec!["seo.titulo.longo"]);
        assert!(pode_publicar(&problemas));

        let excedido = com_post(|p| p.seo.titulo = "ã".repeat(71));
        let problemas = validar_sem_contexto(&excedido);
        assert_eq!(codigos(&problemas), vec!["seo.titulo.excedido"]);
        assert!(!pode_publicar(&problemas));
    }

    #[test]
    fn descricao_vazia_longa_e_excedida() {
        let vazia = com_post(|p| p.seo.descricao = String::new());
        assert!(codigos(&validar_sem_contexto(&vazia)).contains(&"seo.descricao.vazia"));

        let longa = com_post(|p| p.seo.descricao = "d".repeat(156));
        assert_eq!(
            codigos(&validar_sem_contexto(&longa)),
            vec!["seo.descricao.longa"]
        );

        let excedida = com_post(|p| p.seo.descricao = "d".repeat(171));
        assert_eq!(
            codigos(&validar_sem_contexto(&excedida)),
            vec!["seo.descricao.excedida"]
        );
    }

    #[test]
    fn titulo_e_descricao_repetidos_no_site() {
        let documento = demonstracao::post_de_exemplo();
        let titulos = [format!("  {}  ", documento.seo().titulo.to_uppercase())];
        let descricoes = [documento.seo().descricao.clone()];
        let contexto = Contexto {
            titulos_em_uso: &titulos,
            descricoes_em_uso: &descricoes,
            ..Contexto::vazio()
        };
        assert_eq!(
            codigos(&validar(&documento, &contexto)),
            vec!["seo.titulo.repetido", "seo.descricao.repetida"]
        );
    }

    #[test]
    fn slug_vazio_fora_do_formato_e_sem_redirecionamento() {
        let vazio = com_post(|p| p.slug = String::new());
        assert!(codigos(&validar_sem_contexto(&vazio)).contains(&"slug.vazio"));

        let so_simbolos = com_post(|p| p.slug = "!!!".into());
        assert!(codigos(&validar_sem_contexto(&so_simbolos)).contains(&"slug.vazio"));

        let com_acento = com_post(|p| p.slug = "Promoção".into());
        assert_eq!(
            codigos(&validar_sem_contexto(&com_acento)),
            vec!["slug.formato"]
        );

        let documento = demonstracao::post_de_exemplo();
        let sem = Contexto {
            slug_publicado: Some("endereco-antigo"),
            ..Contexto::vazio()
        };
        assert_eq!(
            codigos(&validar(&documento, &sem)),
            vec!["slug.sem-redirecionamento"]
        );

        let redirecionados = ["endereco-antigo".to_string()];
        let com = Contexto {
            slug_publicado: Some("endereco-antigo"),
            redirecionados: &redirecionados,
            ..Contexto::vazio()
        };
        assert_eq!(validar(&documento, &com), vec![]);
    }

    #[test]
    fn home_pode_ter_slug_vazio() {
        let home = demonstracao::documentos()
            .into_iter()
            .find(Documento::eh_home)
            .expect("a fixture tem home");
        assert_eq!(validar_sem_contexto(&home), vec![]);
    }

    #[test]
    fn titulo_de_secao_nao_pula_nivel() {
        let titulo = |nivel: u8| Bloco::Titulo {
            nivel,
            texto: "Seção".into(),
        };

        let certo =
            com_post(|p| p.corpo = vec![titulo(2), titulo(3), titulo(4), titulo(2), titulo(3)]);
        assert_eq!(validar_sem_contexto(&certo), vec![]);

        let comeca_no_tres = com_post(|p| p.corpo = vec![titulo(3)]);
        assert_eq!(
            codigos(&validar_sem_contexto(&comeca_no_tres)),
            vec!["corpo.titulo.nivel"]
        );

        let pula = com_post(|p| p.corpo = vec![titulo(2), titulo(4)]);
        let problemas = validar_sem_contexto(&pula);
        assert_eq!(codigos(&problemas), vec!["corpo.titulo.nivel"]);
        assert_eq!(problemas[0].campo, "corpo[1].nivel");

        let nivel_um = com_post(|p| p.corpo = vec![titulo(1)]);
        assert_eq!(
            codigos(&validar_sem_contexto(&nivel_um)),
            vec!["corpo.titulo.nivel"]
        );
    }

    #[test]
    fn imagem_precisa_de_descricao_de_verdade() {
        let sem_alt = com_post(|p| p.capa.alt = " ".into());
        let problemas = validar_sem_contexto(&sem_alt);
        assert_eq!(codigos(&problemas), vec!["midia.alt.vazio"]);
        assert_eq!(problemas[0].campo, "capa.alt");

        let extensao = com_post(|p| p.capa.alt = "IMG_2041.JPG".into());
        assert_eq!(
            codigos(&validar_sem_contexto(&extensao)),
            vec!["midia.alt.arquivo"]
        );

        // A URL da variante é `/midia/<nome>-<hash>-<largura>.<formato>`.
        let nome_do_arquivo = com_post(|p| {
            let nome = nome_base(&p.capa.variantes[0].url);
            p.capa.alt = nome.replace('-', " ");
        });
        assert_eq!(
            codigos(&validar_sem_contexto(&nome_do_arquivo)),
            vec!["midia.alt.arquivo"]
        );
    }

    #[test]
    fn nome_base_desconta_hash_e_largura() {
        assert_eq!(
            nome_base("/midia/bancada-de-marcenaria-1a2b3c4d-1200.webp?v=2"),
            "bancada-de-marcenaria"
        );
        assert_eq!(nome_base("https://x.com/a/foto.avif"), "foto");
    }

    #[test]
    fn post_exige_autor_completo_e_capa_grande() {
        let sem_bio = com_post(|p| p.autor.bio = String::new());
        assert_eq!(
            codigos(&validar_sem_contexto(&sem_bio)),
            vec!["post.autor.incompleto"]
        );

        let sem_foto = com_post(|p| p.autor.foto.id = String::new());
        assert_eq!(
            codigos(&validar_sem_contexto(&sem_foto)),
            vec!["post.autor.incompleto"]
        );

        let capa_pequena = com_post(|p| p.capa.largura = 1199);
        assert_eq!(
            codigos(&validar_sem_contexto(&capa_pequena)),
            vec!["post.capa.pequena"]
        );
    }

    #[test]
    fn link_e_video_so_de_origem_aceita() {
        let link = |destino: &str| Bloco::Paragrafo {
            trechos: vec![Trecho {
                texto: "clique".into(),
                link: Some(destino.into()),
                ..Trecho::default()
            }],
        };
        let ruim = com_post(|p| {
            p.corpo = vec![
                link("javascript:alert(1)"),
                Bloco::Chamada {
                    texto: "Fale".into(),
                    rotulo: "Contato".into(),
                    url: "//golpe.com".into(),
                },
                Bloco::Video {
                    url: "https://exemplo.com/v.mp4".into(),
                    titulo: "Vídeo".into(),
                },
            ]
        });
        assert_eq!(
            codigos(&validar_sem_contexto(&ruim)),
            vec!["link.invalido", "link.invalido", "video.origem"]
        );

        let bom = com_post(|p| {
            p.corpo = vec![
                link("/contato"),
                link("https://exemplo.com.br"),
                Bloco::Video {
                    url: "https://vimeo.com/123".into(),
                    titulo: "Vídeo".into(),
                },
            ]
        });
        assert_eq!(validar_sem_contexto(&bom), vec![]);
    }

    #[test]
    fn produto_de_catalogo_vale_sem_preco_e_cobra_resumo_ficha_e_modelo() {
        let documento = demonstracao::item_de_exemplo();
        assert_eq!(validar_sem_contexto(&documento), vec![]);
        assert!(documento.tem_modelo_3d());

        let mut ruim = documento.clone();
        if let Conteudo::Item(item) = &mut ruim.conteudo {
            item.resumo = " ".into();
            item.ficha[1].valor = String::new();
            if let Some(modelo) = &mut item.modelo {
                modelo.glb = "https://outro.example/m.glb".into();
            }
            if let Some(acao) = &mut item.acao {
                acao.url = "javascript:alert(1)".into();
            }
        }
        let problemas = validar_sem_contexto(&ruim);
        let achados = codigos(&problemas);
        for codigo in [
            "item.resumo.vazio",
            "item.ficha.incompleta",
            "modelo.arquivo",
            "link.invalido",
        ] {
            assert!(achados.contains(&codigo), "{codigo}: {achados:?}");
        }
    }

    #[test]
    fn produto_exige_variacao_com_preco() {
        let sem = com_produto(|p| p.variacoes.clear());
        assert_eq!(
            codigos(&validar_sem_contexto(&sem)),
            vec!["produto.sem-variacao"]
        );

        let gratis = com_produto(|p| {
            p.variacoes = vec![
                Variacao {
                    preco_centavos: 0,
                    ..p.variacoes[0].clone()
                },
                Variacao {
                    preco_centavos: -100,
                    ..p.variacoes[0].clone()
                },
            ]
        });
        let problemas = validar_sem_contexto(&gratis);
        assert_eq!(codigos(&problemas), vec!["produto.preco", "produto.preco"]);
        assert_eq!(problemas[1].campo, "variacoes[1].precoCentavos");
    }

    #[test]
    fn noindex_e_peso_avisam_sem_barrar() {
        let escondido = com_post(|p| p.seo.indexar = false);
        let contexto = Contexto {
            peso_kb: Some(900),
            orcamento_peso_kb: 500,
            ..Contexto::vazio()
        };
        let problemas = validar(&escondido, &contexto);
        assert_eq!(codigos(&problemas), vec!["seo.noindex", "pagina.peso"]);
        assert!(pode_publicar(&problemas));

        let dentro = Contexto {
            peso_kb: Some(500),
            orcamento_peso_kb: 500,
            ..Contexto::vazio()
        };
        assert_eq!(validar(&demonstracao::post_de_exemplo(), &dentro), vec![]);
    }

    #[test]
    fn data_so_muda_quando_o_conteudo_muda() {
        let antes = demonstracao::post_de_exemplo();
        assert!(!deve_atualizar_data(&antes, &antes.clone()));

        let so_busca = com_post(|p| p.seo.descricao = "Outra descrição para a busca.".into());
        assert!(!deve_atualizar_data(&antes, &so_busca));

        let titulo = com_post(|p| p.titulo.push('!'));
        assert!(deve_atualizar_data(&antes, &titulo));

        let corpo = com_post(|p| {
            p.corpo.push(Bloco::Titulo {
                nivel: 2,
                texto: "Novo".into(),
            })
        });
        assert!(deve_atualizar_data(&antes, &corpo));
    }

    #[test]
    fn upload_le_o_formato_dos_bytes() {
        assert_eq!(
            validar_upload("foto.png", &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]),
            vec![]
        );
        assert_eq!(
            validar_upload("a.png", &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            vec![]
        );
        assert_eq!(validar_upload("a.webp", b"RIFF\0\0\0\0WEBPVP8 "), vec![]);

        // Extensão de imagem com conteúdo de outra coisa.
        assert_eq!(
            codigos(&validar_upload("foto.jpg", b"<svg onload=alert(1)>")),
            vec!["upload.formato"]
        );
        assert_eq!(
            codigos(&validar_upload("vazio.jpg", &[])),
            vec!["upload.formato"]
        );
    }

    #[test]
    fn upload_grande_e_recusado() {
        let mut bytes = vec![0u8; UPLOAD_MAXIMO_BYTES + 1];
        bytes[..3].copy_from_slice(&[0xFF, 0xD8, 0xFF]);
        assert_eq!(
            codigos(&validar_upload("grande.jpg", &bytes)),
            vec!["upload.grande"]
        );
    }

    #[test]
    fn gerador_recusa_documento_que_nao_pode_ser_publicado() {
        let ruim = com_post(|p| p.capa.alt = String::new());
        let erro = exigir_publicavel(&ruim).expect_err("capa sem descrição");
        assert_eq!(erro.codigo, "midia.alt.vazio");
        assert_eq!(erro.campo, "capa.alt");
        assert!(erro.to_string().contains(&ruim.caminho));
    }
}
