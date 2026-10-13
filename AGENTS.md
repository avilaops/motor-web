# motor-web: regras para agentes

Crate Rust da Ávila Ops com as regras de conteúdo usadas pelo CMS e pelo
Lojas. Nicolas Avila (sem acento) é quem decide. Comunicar em português, com
resposta direta e evidência.

O desenho está no repositório do CMS:
[2026-10-09-motor-web-design.md](https://github.com/avilaops/cms.avilaops.com/blob/main/docs/superpowers/specs/2026-10-09-motor-web-design.md).
Mudou o comportamento do crate, aquele documento muda no mesmo trabalho.

## Regras do projeto

- **Este repositório é público.** Nada de dado, nome, foto ou domínio de
  cliente, nem em código, nem em teste, nem em fixture. O site de demonstração
  é fictício.
- **Sem banco, rede, relógio nem estado global.** Data e hora entram por
  parâmetro. Função pura: mesma entrada, mesma saída, byte a byte.
- **Nenhum `HashMap` é iterado para gerar texto.** A saída é comparada com
  referência; ordem instável quebra o teste e o cache de quem consome.
- **Sem `unwrap`, `expect` nem `panic!` fora de teste.** Entrada malformada
  vira `Problema` ou `Err`, nunca pânico. `tests/entradas_geradas.rs` cobra
  isso.
- **Validação devolve `Vec<Problema>`, não `Result`.** Os avisos precisam
  chegar a quem edita mesmo quando a página pode ser publicada.
- **O código de um `Problema` é contrato.** Painel, API e conector reconhecem a
  regra por ele. Renomear código é mudança incompatível.
- **Texto de quem edita nunca é marcação.** Tudo que vai para HTML ou XML passa
  por `escape::marcacao`; JSON dentro de `<script>` passa por
  `escape::json_para_script`. Link só `http(s)`, caminho do próprio site ou
  `mailto:` com um destinatário; vídeo só YouTube e Vimeo.
- **O corpo não gera `<h1>`.** O `<h1>` é o título do documento, e quem o põe
  na página é o template de quem consome.
- **O motor não conhece cliente nem rota.** O caminho de cada documento vem de
  quem chama. Nada de `if site == "x"`.
- **`aggregateRating` só com avaliação do próprio site** e com total maior que
  zero.
- **Regra nova entra com teste dos dois lados:** um caso que passa e um que
  barra, com o código conferido.
- **Mudou a saída, a referência muda no mesmo commit**, e o `git diff` de
  `tests/snapshots` é lido antes de commitar. Referência aceita sem ler não
  prova nada.
- **Dinheiro em centavos, `i64`.** Português nos nomes e comentários.

## Antes de entregar

`cargo fmt --check`, `cargo clippy --all-features --all-targets -- -D warnings`
e `cargo test --all-features`. Rodar também `cargo test` sem a *feature*
`imagem`: quem só valida compila assim.

## Quem consome

CMS e Lojas fixam o crate por commit (`rev` no `Cargo.toml`). Mudança aqui não
chega a eles sozinha: é preciso atualizar o `rev` em cada um.
