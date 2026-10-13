# motor-web

Regras de conteúdo compartilhadas pelo CMS e pelo Lojas da Ávila Ops. O crate
recebe conteúdo e devolve os problemas que impedem a publicação e os artefatos
da página: cabeçalho, JSON-LD, HTML do corpo e das imagens, sitemaps,
`robots.txt` e `llms.txt`.

Não tem banco, rede, relógio nem estado global. Mesma entrada, mesma saída.

## Uso

```toml
[dependencies]
motor-web = { git = "https://github.com/avilaops/motor-web", rev = "<commit>" }
# Para gerar variantes de imagem:
# motor-web = { git = "...", rev = "<commit>", features = ["imagem"] }
```

O consumo é por commit fixo. Mudou o motor, quem consome atualiza o `rev`.

## Módulos

| Módulo | O que faz |
|---|---|
| `tipos` | Contrato do conteúdo: `Site`, `Documento`, `Pagina`, `Post`, `Product`, `Bloco`, `Midia` |
| `validacao` | `validar`, `pode_publicar`, `normalizar_slug`, `validar_upload`, `deve_atualizar_data` |
| `seo` | `canonica`, `montar_cabecalho`, `Cabecalho::para_html` |
| `schema` | `montar_json_ld` |
| `conteudo` | `renderizar_corpo`, `texto_do_corpo`, `contar_palavras` |
| `midia` | `renderizar_midia`, `url_principal` |
| `cor` | `conferir_cor`: a cor de destaque do site e da página, com contraste mínimo contra o branco |
| `descoberta` | `gerar_sitemaps`, `gerar_robots`, `gerar_llms`, `gerar_llms_completo` |
| `secoes` | Abertura de página e seções (cartões, destaque, depoimentos, números, passos, planos, galeria, logos, faixa, contato): `renderizar_abertura` e `ESTILO_DAS_SECOES` |
| `imagem` | `processar_imagem` (só com a *feature* `imagem`) |
| `demonstracao` | Site fictício usado nos testes e como semente do CMS |

## Exemplo

```rust
use motor_web::conteudo::renderizar_corpo;
use motor_web::seo::montar_cabecalho;
use motor_web::validacao::{Contexto, pode_publicar, validar};

let problemas = validar(&documento, &Contexto::vazio());
if !pode_publicar(&problemas) {
    // Devolver `problemas` a quem edita: cada um traz código, campo e mensagem.
}

let cabecalho = montar_cabecalho(&documento, &site)?;
let head = cabecalho.para_html();           // <title>, <meta>, <link>, JSON-LD
let corpo = renderizar_corpo(documento.corpo());
```

`Contexto` leva o que a regra precisa saber do resto do site: títulos e
descrições já usados, o slug com que o documento está publicado, os
redirecionamentos e o peso medido da página.

## Desenvolvimento

```bash
cargo fmt --check
```

```bash
cargo clippy --all-features --all-targets -- -D warnings
```

```bash
cargo test --all-features
```

A saída do site de demonstração é comparada com as referências em
`tests/snapshots`. Mudança intencional na saída:

```bash
cargo insta review
```

Sem o `cargo-insta` instalado, rode os testes com `INSTA_UPDATE=always` e
confira o `git diff` das referências antes de commitar.

A *feature* `imagem` compila codificadores de AVIF e WebP e precisa de um
compilador C. A primeira compilação leva alguns minutos.

`processar_imagem` aceita JPG, PNG e WebP. AVIF é só formato de saída.

## Desenho

A spec fica no repositório do CMS:
[2026-10-09-motor-web-design.md](https://github.com/avilaops/cms.avilaops.com/blob/main/docs/superpowers/specs/2026-10-09-motor-web-design.md).
