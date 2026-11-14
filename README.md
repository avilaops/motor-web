# motor-web

Regras de conteúdo compartilhadas pelo CMS e pelo Lojas da Ávila Ops. O crate
recebe conteúdo e devolve os problemas que impedem a publicação e os artefatos
da página: cabeçalho, JSON-LD, HTML do corpo e das imagens, sitemaps,
`robots.txt` e `llms.txt`.

Não tem banco, rede, relógio nem estado global. Mesma entrada, mesma saída.

## Publicação de MDX e Markdown com Node

O pacote opcional `@avilaops/publicacao-texto` converte o HTML do artefato de
publicação em versões `.mdx` e `.md`. Quem usa apenas o crate Rust continua
usando Cargo; o pacote Node não altera suas dependências ou contratos.

```bash
npm ci --ignore-scripts
npm test
node bin/publicar-texto.mjs --diretorio caminho/do/artefato --origem https://oficina-exemplo.example
```

Execute o comando depois do build e antes do deploy. Ele percorre as páginas
HTML, gera as versões textuais, acrescenta links de descoberta e registra a
cobertura em `publicacao-texto.json`. A saída inclui regras de `_headers` para
Cloudflare Pages, com tipo de conteúdo e `noindex` nas cópias textuais. O
`llms.txt` existente é preservado e recebe uma seção com as versões Markdown.

Uma rota `/guias/pecas/` gera `/guias/pecas/index.mdx` e
`/guias/pecas/index.md`; arquivos legados, como `/guia.html`, recebem a extensão
adicional (`/guia.html.mdx`). O endereço de saída vem da rota original, mesmo
quando duas páginas têm a mesma canônica.

O comando exclui rotas privadas, páginas de erro e conteúdo marcado como
`noindex` ou `none`, considerando todas as metatags `robots` e as regras
`X-Robots-Tag` de `_headers`. São respeitados padrões de URL, hosts,
placeholders, splats e a ordem de remoção e adição desses cabeçalhos.
Para arquivos `.html`, uma restrição na URL com ou sem extensão exclui a
página. Os nomes do filesystem são codificados por segmento; percentuais
literais não mudam o diretório de saída.

Um shell de aplicação sem texto renderizado faz a etapa falhar,
pois não comprova a cobertura do conteúdo. Arquivos manuais não são
sobrescritos, symlinks não são seguidos e apenas saídas reconhecidas pelo
manifesto anterior podem ser retiradas quando o HTML deixa de existir ou
passa a ser excluído. Nesse caso, seus links de descoberta gerados também
são retirados do HTML. A edição preserva scripts e comentários existentes.

Para um adaptador de aplicativo, `converterHtml(html, { url, formato })`
devolve conteúdo, metadados, links e a decisão de indexação. O adaptador deve
preservar autorização, isolamento de loja e política de cache do HTML. A
função não acessa rede, disco ou relógio, e nunca usa texto editorial como
código MDX. HTML editorial é escapado nos dois formatos. Código literal em
cercas e inline é preservado a partir da árvore Markdown; código indentado
não é tratado como uma região segura no MDX.

## Descoberta pública para agentes

O mesmo pacote Node oferece um gerador puro de catálogos ARD, um publicador
de diretório e um adaptador opcional WebMCP de leitura. Execute depois do
build, antes do deploy:

```bash
node bin/publicar-descoberta.mjs --diretorio dist --origem https://empresa.example
```

Para excluir cópias de rotas publicadas por outro sistema, repita
`--excluir-prefixo /rota/`. Esses arquivos ficam intactos e fora do catálogo.
O comando preserva `robots.txt`, redirects, scripts e JSON-LD existentes;
recusa colisões com arquivos manuais e não segue symlinks.

Veja [o contrato e os limites da descoberta pública](docs/descoberta-publica.md).

## Uso do crate Rust

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
