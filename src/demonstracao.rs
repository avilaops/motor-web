//! Site de demonstração: uma marcenaria fictícia. É a fixture dos testes do
//! crate e a semente do site de demonstração do CMS. Nenhum nome de cliente
//! real entra aqui.

use chrono::{DateTime, NaiveDate, Utc};

use crate::descoberta::PaginaDeCategoria;
use crate::tipos::{
    Autor, Avaliacoes, Bloco, Categoria, Conteudo, Direitos, Disponibilidade, Documento, Endereco,
    Formato, ItemTrilha, Midia, Organizacao, Pagina, Pergunta, Post, Product, Seo, Site, TipoPost,
    Trecho, Variacao, Variante,
};

const ORIGEM: &str = "https://oficina-exemplo.example";

fn dia(ano: i32, mes: u32, dia: u32) -> DateTime<Utc> {
    NaiveDate::from_ymd_opt(ano, mes, dia)
        .and_then(|data| data.and_hms_opt(12, 0, 0))
        .map(|momento| momento.and_utc())
        .unwrap_or_default()
}

/// Mídia já processada, com as variantes que o motor geraria para a largura.
fn midia(nome: &str, alt: &str, largura: u32, altura: u32) -> Midia {
    let mut larguras: Vec<u32> = [480, 768, 1200, 1920]
        .into_iter()
        .filter(|l| *l <= largura)
        .collect();
    if larguras.last() != Some(&largura) && largura < 1920 {
        larguras.push(largura);
    }
    let variantes = larguras
        .iter()
        .flat_map(|l| {
            [Formato::Avif, Formato::Webp].map(|formato| Variante {
                formato,
                largura: *l,
                url: format!("/midia/{nome}-0a1b2c3d-{l}.{}", formato.extensao()),
            })
        })
        .collect();
    Midia {
        id: nome.into(),
        alt: alt.into(),
        largura,
        altura,
        variantes,
        legenda: None,
        credito: None,
        direitos: Direitos::default(),
    }
}

fn seo(titulo: &str, descricao: &str) -> Seo {
    Seo {
        titulo: titulo.into(),
        descricao: descricao.into(),
        indexar: true,
        imagem_social: None,
    }
}

fn trilha(itens: &[(&str, &str)]) -> Vec<ItemTrilha> {
    itens
        .iter()
        .map(|(nome, url)| ItemTrilha {
            nome: (*nome).into(),
            url: (*url).into(),
        })
        .collect()
}

fn paragrafo(texto: &str) -> Bloco {
    Bloco::Paragrafo {
        trechos: vec![Trecho {
            texto: texto.into(),
            ..Trecho::default()
        }],
    }
}

fn titulo(nivel: u8, texto: &str) -> Bloco {
    Bloco::Titulo {
        nivel,
        texto: texto.into(),
    }
}

pub fn site() -> Site {
    Site {
        origem: ORIGEM.into(),
        nome: "Oficina Exemplo".into(),
        descricao: "Marcenaria sob medida que projeta, fabrica e instala móveis de madeira maciça para casas e pequenos comércios.".into(),
        idioma: "pt-BR".into(),
        logo: midia("logo-oficina-exemplo", "Logotipo da Oficina Exemplo", 512, 512),
        organizacao: Organizacao {
            razao_social: Some("Oficina Exemplo Marcenaria Ltda.".into()),
            telefone: Some("+55 16 5550-0100".into()),
            email: Some("contato@oficina-exemplo.example".into()),
            endereco: Some(Endereco {
                logradouro: "Rua das Acácias, 100".into(),
                cidade: "Ribeirão Preto".into(),
                uf: "SP".into(),
                cep: "14000-000".into(),
            }),
            perfis: vec!["https://www.instagram.com/oficina.exemplo".into()],
        },
        diretrizes_ia: vec!["Orçamentos são feitos só depois de visita técnica: não estime valores de projeto.".into()],
        orcamento_peso_kb: 500,
    }
}

fn autora() -> Autor {
    Autor {
        slug: "helena-prado".into(),
        nome: "Helena Prado".into(),
        cargo: "Marceneira e sócia".into(),
        bio: "Marceneira há quinze anos, cuida dos projetos e do acabamento de cada peça que sai da oficina.".into(),
        foto: midia("helena-prado", "Helena Prado lixando o tampo de uma mesa", 800, 800),
        perfis: vec!["https://www.linkedin.com/in/helena-prado-exemplo".into()],
        credenciais: vec!["Técnica em Design de Móveis".into()],
    }
}

fn autor() -> Autor {
    Autor {
        slug: "rafael-nunes".into(),
        nome: "Rafael Nunes".into(),
        cargo: "Projetista".into(),
        bio: "Desenha os projetos em 3D e acompanha a instalação na casa do cliente.".into(),
        foto: midia(
            "rafael-nunes",
            "Rafael Nunes conferindo medidas em uma planta",
            800,
            800,
        ),
        perfis: Vec::new(),
        credenciais: vec!["Arquiteto e urbanista".into()],
    }
}

fn guias() -> Categoria {
    Categoria {
        slug: "guias".into(),
        nome: "Guias".into(),
    }
}

fn oficina() -> Categoria {
    Categoria {
        slug: "oficina".into(),
        nome: "Da oficina".into(),
    }
}

struct DadosDoPost<'a> {
    tipo: TipoPost,
    slug: &'a str,
    titulo: &'a str,
    resumo: &'a str,
    alt_da_capa: &'a str,
    corpo: Vec<Bloco>,
    autor: Autor,
    categoria: Categoria,
    tags: &'a [&'a str],
    publicado_em: DateTime<Utc>,
    atualizado_em: DateTime<Utc>,
    busca: (&'a str, &'a str),
}

fn post(dados: DadosDoPost) -> Documento {
    let caminho = format!("/blog/{}", dados.slug);
    Documento {
        conteudo: Conteudo::Post(Post {
            tipo: dados.tipo,
            slug: dados.slug.into(),
            titulo: dados.titulo.into(),
            resumo: dados.resumo.into(),
            capa: midia(
                &format!("capa-{}", dados.slug),
                dados.alt_da_capa,
                1920,
                1080,
            ),
            corpo: dados.corpo,
            autor: dados.autor,
            categoria: dados.categoria,
            tags: dados.tags.iter().map(|t| (*t).into()).collect(),
            publicado_em: dados.publicado_em,
            atualizado_em: dados.atualizado_em,
            seo: seo(dados.busca.0, dados.busca.1),
            trilha: trilha(&[
                ("Início", "/"),
                ("Blog", "/blog"),
                (dados.titulo, caminho.as_str()),
            ]),
        }),
        caminho,
    }
}

/// Um artigo simples e válido, para os testes que alteram um campo por vez.
pub fn post_de_exemplo() -> Documento {
    post(DadosDoPost {
        tipo: TipoPost::Artigo,
        slug: "como-escolher-a-madeira-da-mesa",
        titulo: "Como escolher a madeira da mesa de jantar",
        resumo: "Dureza, veio e acabamento: o que pesa na escolha da madeira de uma mesa que vai durar décadas.",
        alt_da_capa: "Tampo de mesa em madeira maciça com o veio aparente",
        corpo: vec![
            paragrafo(
                "A mesa de jantar é o móvel que mais apanha em uma casa. A madeira certa aguenta o uso sem perder a cara.",
            ),
            titulo(2, "O que olhar na madeira"),
            Bloco::Lista {
                ordenada: false,
                itens: vec![
                    "Dureza: madeiras duras marcam menos.".into(),
                    "Veio: define o desenho do tampo.".into(),
                    "Origem: peça sempre o documento de procedência.".into(),
                ],
            },
            titulo(3, "Acabamento"),
            Bloco::Paragrafo {
                trechos: vec![
                    Trecho {
                        texto: "O acabamento protege e muda a cor. Veja ".into(),
                        ..Trecho::default()
                    },
                    Trecho {
                        texto: "o guia de acabamentos".into(),
                        link: Some("/blog/guia-de-acabamentos-para-madeira".into()),
                        ..Trecho::default()
                    },
                    Trecho {
                        texto: " antes de decidir.".into(),
                        ..Trecho::default()
                    },
                ],
            },
            Bloco::Imagem {
                midia: Midia {
                    legenda: Some("Amostras de acabamento sobre a mesma madeira.".into()),
                    credito: Some("Oficina Exemplo".into()),
                    direitos: Direitos {
                        autoria: Some("Helena Prado".into()),
                        aviso: Some("© 2026 Oficina Exemplo".into()),
                        licenca: Some("https://creativecommons.org/licenses/by/4.0/".into()),
                        aquisicao: Some("/contato".into()),
                    },
                    ..midia(
                        "amostras-de-acabamento",
                        "Quatro amostras de madeira com acabamentos diferentes",
                        1600,
                        900,
                    )
                },
            },
            Bloco::Chamada {
                texto: "Quer ver as madeiras de perto?".into(),
                rotulo: "Agende uma visita".into(),
                url: "/contato".into(),
            },
        ],
        autor: autora(),
        categoria: guias(),
        tags: &["madeira", "mesa de jantar"],
        publicado_em: dia(2026, 8, 12),
        atualizado_em: dia(2026, 9, 3),
        busca: (
            "Como escolher a madeira da mesa de jantar",
            "Dureza, veio e acabamento: veja o que considerar para escolher a madeira de uma mesa de jantar que dure décadas.",
        ),
    })
}

/// Um produto válido com três variações, para os testes que alteram um campo
/// por vez.
pub fn produto_de_exemplo() -> Documento {
    Documento {
        caminho: "/produtos/banco-de-madeira-macica".into(),
        conteudo: Conteudo::Produto(Product {
            slug: "banco-de-madeira-macica".into(),
            nome: "Banco de madeira maciça".into(),
            descricao: "Banco de madeira maciça com encaixes aparentes, feito sob encomenda em três comprimentos.".into(),
            marca: Some("Oficina Exemplo".into()),
            imagens: vec![
                midia("banco-de-frente", "Banco de madeira maciça visto de frente", 1920, 1280),
                midia("banco-encaixe", "Detalhe do encaixe entre o assento e o pé do banco", 1600, 1067),
            ],
            variacoes: vec![
                Variacao {
                    sku: "BANCO-100".into(),
                    nome: "1,00 m".into(),
                    preco_centavos: 89_000,
                    disponibilidade: Disponibilidade::EmEstoque,
                    gtin: Some("7890000000017".into()),
                },
                Variacao {
                    sku: "BANCO-140".into(),
                    nome: "1,40 m".into(),
                    preco_centavos: 119_000,
                    disponibilidade: Disponibilidade::SobEncomenda,
                    gtin: None,
                },
                Variacao {
                    sku: "BANCO-180".into(),
                    nome: "1,80 m".into(),
                    preco_centavos: 149_050,
                    disponibilidade: Disponibilidade::SemEstoque,
                    gtin: None,
                },
            ],
            avaliacoes: Some(Avaliacoes { media: 4.8, total: 23 }),
            politica_devolucao_url: "/trocas-e-devolucoes".into(),
            atualizado_em: dia(2026, 9, 20),
            seo: seo(
                "Banco de madeira maciça sob encomenda",
                "Banco de madeira maciça com encaixes aparentes, em três comprimentos. Feito sob encomenda pela Oficina Exemplo.",
            ),
            trilha: trilha(&[("Início", "/"), ("Produtos", "/produtos"), ("Banco de madeira maciça", "/produtos/banco-de-madeira-macica")]),
        }),
    }
}

fn pagina(
    slug: &str,
    titulo_da_pagina: &str,
    corpo: Vec<Bloco>,
    capa: Option<Midia>,
    busca: (&str, &str),
) -> Documento {
    let caminho = format!("/{slug}");
    let trilha = if slug.is_empty() {
        Vec::new()
    } else {
        trilha(&[("Início", "/"), (titulo_da_pagina, caminho.as_str())])
    };
    Documento {
        conteudo: Conteudo::Pagina(Pagina {
            slug: slug.into(),
            titulo: titulo_da_pagina.into(),
            corpo,
            capa,
            publicado_em: dia(2026, 7, 1),
            atualizado_em: dia(2026, 9, 15),
            seo: seo(busca.0, busca.1),
            trilha,
        }),
        caminho,
    }
}

/// O site inteiro: home, três páginas, cinco posts e dois produtos. A home é
/// o primeiro item.
pub fn documentos() -> Vec<Documento> {
    vec![
        pagina(
            "",
            "Móveis de madeira maciça feitos sob medida",
            vec![
                paragrafo("Projetamos, fabricamos e instalamos móveis de madeira maciça para casas e pequenos comércios."),
                titulo(2, "Como trabalhamos"),
                Bloco::Lista {
                    ordenada: true,
                    itens: vec!["Visita e medição.".into(), "Projeto em 3D.".into(), "Fabricação e instalação.".into()],
                },
            ],
            None,
            (
                "Oficina Exemplo: marcenaria sob medida em madeira maciça",
                "Marcenaria sob medida em Ribeirão Preto. Projeto em 3D, fabricação própria e instalação de móveis de madeira maciça.",
            ),
        ),
        pagina(
            "sobre",
            "Sobre a oficina",
            vec![
                paragrafo("A Oficina Exemplo nasceu em uma garagem e hoje ocupa um galpão com seis pessoas."),
                Bloco::Citacao {
                    texto: "Móvel bom é o que o neto herda.".into(),
                    fonte: Some("Helena Prado".into()),
                },
            ],
            Some(midia("galpao-da-oficina", "Galpão da oficina com bancadas e tábuas empilhadas", 1920, 1080)),
            (
                "Sobre a Oficina Exemplo",
                "Conheça a história, a equipe e o jeito de trabalhar da Oficina Exemplo, marcenaria de móveis sob medida em madeira maciça.",
            ),
        ),
        pagina(
            "servicos",
            "Serviços",
            vec![
                titulo(2, "O que fazemos"),
                Bloco::Tabela {
                    cabecalho: vec!["Serviço".into(), "Prazo médio".into()],
                    linhas: vec![
                        vec!["Mesa de jantar".into(), "30 dias".into()],
                        vec!["Estante sob medida".into(), "45 dias".into()],
                        vec!["Restauro".into(), "A combinar".into()],
                    ],
                },
            ],
            None,
            (
                "Serviços de marcenaria sob medida",
                "Mesas, estantes, cozinhas e restauro de móveis em madeira maciça. Veja os serviços e os prazos médios da Oficina Exemplo.",
            ),
        ),
        pagina(
            "contato",
            "Contato",
            vec![paragrafo("Atendemos de segunda a sexta, das 8h às 18h, com hora marcada.")],
            None,
            (
                "Fale com a Oficina Exemplo",
                "Agende uma visita ou peça um orçamento de móveis sob medida. Atendimento de segunda a sexta, com hora marcada.",
            ),
        ),
        post_de_exemplo(),
        post(DadosDoPost {
            tipo: TipoPost::GuiaTecnico,
            slug: "guia-de-acabamentos-para-madeira",
            titulo: "Guia de acabamentos para madeira",
            resumo: "Óleo, cera, verniz e laca: como cada acabamento se comporta e quando usar.",
            alt_da_capa: "Pincel aplicando óleo sobre uma tábua de madeira clara",
            corpo: vec![
                paragrafo("Cada acabamento muda a cor, o toque e a manutenção da peça."),
                titulo(2, "Comparação"),
                Bloco::Tabela {
                    cabecalho: vec!["Acabamento".into(), "Resistência".into(), "Manutenção".into()],
                    linhas: vec![
                        vec!["Óleo".into(), "Média".into(), "Reaplicar todo ano".into()],
                        vec!["Verniz".into(), "Alta".into(), "Lixar para refazer".into()],
                    ],
                },
                titulo(2, "Aplicação"),
                Bloco::Lista {
                    ordenada: true,
                    itens: vec!["Lixe no sentido do veio.".into(), "Limpe o pó.".into(), "Aplique em camadas finas.".into()],
                },
                Bloco::Video {
                    url: "https://www.youtube.com/watch?v=exemplo0001".into(),
                    titulo: "Aplicação de óleo em tampo de mesa".into(),
                },
            ],
            autor: autora(),
            categoria: guias(),
            tags: &["acabamento", "verniz", "óleo"],
            publicado_em: dia(2026, 8, 26),
            atualizado_em: dia(2026, 9, 10),
            busca: (
                "Guia de acabamentos para madeira: óleo, cera e verniz",
                "Compare óleo, cera, verniz e laca: resistência, manutenção e o passo a passo de aplicação de cada acabamento em madeira.",
            ),
        }),
        post(DadosDoPost {
            tipo: TipoPost::Artigo,
            slug: "perguntas-sobre-moveis-sob-medida",
            titulo: "Perguntas frequentes sobre móveis sob medida",
            resumo: "Prazo, garantia e transporte: as dúvidas que mais ouvimos antes de um projeto.",
            alt_da_capa: "Projetista mostrando um desenho de estante a um casal",
            corpo: vec![
                paragrafo("Reunimos as dúvidas que aparecem em quase toda primeira conversa."),
                Bloco::Perguntas {
                    itens: vec![
                        Pergunta {
                            pergunta: "Quanto tempo leva um projeto?".into(),
                            resposta: "De 30 a 60 dias, da aprovação do desenho à instalação.".into(),
                        },
                        Pergunta {
                            pergunta: "Os móveis têm garantia?".into(),
                            resposta: "Sim, cinco anos para estrutura e um ano para ferragens.".into(),
                        },
                    ],
                },
            ],
            autor: autor(),
            categoria: guias(),
            tags: &["sob medida"],
            publicado_em: dia(2026, 9, 2),
            atualizado_em: dia(2026, 9, 2),
            busca: (
                "Móveis sob medida: perguntas frequentes",
                "Prazo, garantia, transporte e pagamento: respostas diretas para as dúvidas mais comuns sobre móveis sob medida.",
            ),
        }),
        post(DadosDoPost {
            tipo: TipoPost::Artigo,
            slug: "como-nasce-uma-estante",
            titulo: "Como nasce uma estante na nossa oficina",
            resumo: "Do desenho à instalação, as etapas de uma estante sob medida.",
            alt_da_capa: "Estante de madeira em montagem sobre a bancada",
            corpo: vec![
                paragrafo("Uma estante passa por cinco mãos antes de chegar à sala do cliente."),
                titulo(2, "Do desenho ao corte"),
                paragrafo("O projeto em 3D vira uma lista de corte, e cada tábua é escolhida pelo veio."),
            ],
            autor: autor(),
            categoria: oficina(),
            tags: &["estante", "bastidores"],
            publicado_em: dia(2026, 9, 16),
            atualizado_em: dia(2026, 9, 16),
            busca: (
                "Como nasce uma estante sob medida",
                "Veja as etapas de fabricação de uma estante sob medida, do projeto em 3D ao corte, montagem e instalação.",
            ),
        }),
        post(DadosDoPost {
            tipo: TipoPost::Artigo,
            slug: "cuidados-com-moveis-de-madeira",
            titulo: "Cuidados com móveis de madeira no dia a dia",
            resumo: "Limpeza, sol e umidade: hábitos simples que fazem o móvel durar.",
            alt_da_capa: "Pano macio limpando o tampo de um aparador de madeira",
            corpo: vec![
                paragrafo("Madeira maciça é resistente, mas reage a sol, água e calor."),
                titulo(2, "Limpeza"),
                paragrafo("Pano macio e levemente úmido. Produto com silicone deixa a superfície escorregadia e mancha."),
                titulo(2, "Sol e umidade"),
                paragrafo("Luz direta clareia a madeira, e a mudança é desigual se houver objetos sobre o tampo."),
            ],
            autor: autora(),
            categoria: oficina(),
            tags: &["manutenção"],
            publicado_em: dia(2026, 9, 30),
            atualizado_em: dia(2026, 10, 2),
            busca: (
                "Cuidados com móveis de madeira maciça",
                "Como limpar e proteger móveis de madeira maciça do sol, da umidade e do calor para que durem muitos anos.",
            ),
        }),
        produto_de_exemplo(),
        Documento {
            caminho: "/produtos/tabua-de-corte".into(),
            conteudo: Conteudo::Produto(Product {
                slug: "tabua-de-corte".into(),
                nome: "Tábua de corte".into(),
                descricao: "Tábua de corte em madeira de topo, com acabamento em óleo mineral próprio para alimentos.".into(),
                marca: Some("Oficina Exemplo".into()),
                imagens: vec![midia("tabua-de-corte-vista-de-cima", "Tábua de corte em madeira de topo vista de cima", 1920, 1280)],
                variacoes: vec![Variacao {
                    sku: "TABUA-01".into(),
                    nome: "Única".into(),
                    preco_centavos: 18_990,
                    disponibilidade: Disponibilidade::EmEstoque,
                    gtin: None,
                }],
                avaliacoes: None,
                politica_devolucao_url: "/trocas-e-devolucoes".into(),
                atualizado_em: dia(2026, 9, 22),
                seo: seo(
                    "Tábua de corte em madeira de topo",
                    "Tábua de corte artesanal em madeira de topo, com acabamento em óleo mineral próprio para contato com alimentos.",
                ),
                trilha: trilha(&[("Início", "/"), ("Produtos", "/produtos"), ("Tábua de corte", "/produtos/tabua-de-corte")]),
            }),
        },
    ]
}

/// As listagens de categoria do blog.
pub fn categorias() -> Vec<PaginaDeCategoria> {
    vec![
        PaginaDeCategoria {
            caminho: "/blog/categoria/guias".into(),
            nome: "Guias".into(),
            atualizado_em: dia(2026, 9, 10),
        },
        PaginaDeCategoria {
            caminho: "/blog/categoria/oficina".into(),
            nome: "Da oficina".into(),
            atualizado_em: dia(2026, 10, 2),
        },
    ]
}
