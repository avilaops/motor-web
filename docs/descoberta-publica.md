# Descoberta pública • Node e navegador

O gerador descreve páginas e recursos públicos já existentes. Não cria um
servidor MCP, agente A2A ou transação. Não acrescenta avaliações, preços ou
tipos Schema.org ao conteúdo. JSON-LD existente é lido como metadado e
preservado no HTML.

## Gerador puro

```js
import { criarCatalogoPublico } from '@avilaops/publicacao-texto/descoberta';

const catalogos = criarCatalogoPublico({
  origem: 'https://empresa.example',
  nome: 'Empresa fictícia',
  descricao: 'Informações públicas da empresa.',
  paginas: [
    { url: '/', titulo: 'Início' },
    { url: '/sobre', titulo: 'Sobre' },
  ],
  recursos: [
    {
      url: '/dados-publicos.json',
      nome: 'Dados públicos',
      tipo: 'application/json',
      consultas: ['Consultar dados públicos da empresa', 'Encontrar informações publicadas'],
    },
  ],
});
```

O retorno contém `ard`, `aiCatalogo`, `jsonArd` e `jsonAiCatalogo`. As versões
em texto terminam com uma quebra de linha. Não há I/O, rede, relógio ou
estado global. A identidade de cada entrada deriva de sua URL e MIME, sem
depender da posição da entrada. A ordem da entrada não altera os bytes.

`origem` precisa ser uma origem HTTP(S), sem caminho, consulta ou
credenciais. Páginas e recursos precisam pertencer à mesma origem. URLs
com consulta e caminhos de autenticação, painel, API, carrinho ou checkout
são recusados. O integrador é responsável por confirmar que um recurso
existe e corresponde ao MIME anunciado. `schemas` em uma página é metadado
opcional do publicador; o gerador não o transforma em recurso ou ação.

O manifesto atual usa a definição `ArdManifest` do
[schema ARD](https://github.com/ards-project/ard-spec/blob/main/spec/schemas/ard-entry.schema.json).
A compatibilidade `aiCatalogo` usa `specVersion: "1.0"`, `host.displayName`
e o domínio em `host.identifier`, sem anunciar uma identidade DID inexistente, conforme o
[schema ai-catalog predecessor](https://github.com/ards-project/ard-spec/blob/main/spec/schemas/ai-catalog.schema.json).
Cada entrada tem `identifier`, `displayName`, `type`, `url` e duas a cinco
consultas. Páginas anunciam o MIME real `text/html`. Esse MIME passa pelo
JSON Schema; o teste semântico adicional do Lighthouse pode emitir aviso
de tipo genérico. Isso não é um registro de tipo WebMCP ou de servidor MCP.

## Publicador de arquivos

```js
import { publicarDescoberta } from '@avilaops/publicacao-texto/descoberta/arquivos';

await publicarDescoberta({
  diretorio: 'dist',
  origem: 'https://empresa.example',
  excluirPrefixos: ['/catalogo-dinamico/', '/area-legada/'],
});
```

Também há o bin `publicar-descoberta`, ou
`node bin/publicar-descoberta.mjs --diretorio dist --origem https://empresa.example`.
As opções `--nome`, `--descricao` e `--excluir-prefixo` são opcionais. Esta
última pode ser repetida. Um prefixo é aplicado por fronteira de segmento:
`/catalogo` cobre `/catalogo/peca`, sem cobrir `/catalogos`.

O adaptador usa HTML renderizado, metatags e regras `X-Robots-Tag` de
`_headers`. Ignora páginas de erro, shells, redirecionamento por metatag,
rotas privadas e `noindex`. Prefixos excluídos ficam byte a byte intactos.
Outras páginas que deixem de ser públicas perdem somente as tags geradas
por este adaptador.

São emitidos `/.well-known/ard.json`, `/ai-catalog.json`,
`/.well-known/ai-catalog.json`, o bootstrap `/descoberta-publica.mjs` e
`/descoberta-navegador.mjs`. `descoberta-publicacao.json` registra cobertura,
problemas e hashes das saídas. A repetição não duplica tags. Arquivos
manuais ou saídas geradas posteriormente editadas causam falha antes da
escrita. Um catálogo ARD manual com outro destino também exige conciliação
explícita. Cabeçalhos existentes são preservados e recebem um bloco de
Content-Type para as saídas.

## WebMCP como melhoria progressiva

O bootstrap só importa o adaptador quando encontra a API e este opt-in
explícito no `head`:

```html
<meta name="descoberta-publica" content="true" data-descoberta-publica="true">
```

Para aplicativos, o ciclo de vida pode ser controlado diretamente:

```js
import { instalarFerramentasPublicas } from '@avilaops/publicacao-texto/descoberta/navegador';

const controlador = new AbortController();
const instalacao = await instalarFerramentasPublicas({
  documento: document,
  modelContext: document.modelContext ?? navigator.modelContext,
  localizacao: location,
  controlador,
});
// Na saída da página ou na mudança para uma área privada:
instalacao.desinstalar();
```

O integrador só deve inserir o opt-in depois de verificar visibilidade,
sessão, tenant e política de indexação da página. O adaptador verifica
novamente caminho, marcador e `noindex` em cada execução. Nunca lê valores
de formulário, storage ou scripts de sessão e nunca faz requisições.

As operações são `consultar_pagina_publica`, com entrada `{}`, e
`localizar_links_publicos`, com `{ consulta, limite? }`. A primeira retorna
URL, título, descrição, JSON-LD parseável e códigos de problema. A segunda
retorna `{ links: [{ titulo, url }] }` da navegação pública da página,
resolvendo links relativos conforme o documento. Links externos, privados,
ocultos e com consulta são excluídos. A consulta tem até 120 caracteres e o
limite vai de 1 a 30, com padrão 10. Entradas inválidas, cancelamento ou
contexto privado retornam `{ erro: { codigo } }`.

O adaptador recebe o `AbortController` e passa seu sinal no registro. A
API atual `document.modelContext` é preferida; a anterior
`navigator.modelContext` é aceita. Registros repetidos no mesmo documento
compartilham a instalação. `abort`, `pagehide` e navegação para contexto
privado removem a disponibilidade; o bootstrap reinstala após restauração
do cache de navegação. Sem API ou sem opt-in, o fluxo existente continua.

As assinaturas seguem o
[draft WebMCP](https://webmachinelearning.github.io/webmcp/) e a
[orientação do Chrome](https://developer.chrome.com/docs/modern-web-guidance/guides/webmcp/agentic-javascript-tools).
No draft consultado, `execute` retorna um valor serializável e recebe o
sinal de cancelamento da execução. Não há envelope de transporte MCP.
