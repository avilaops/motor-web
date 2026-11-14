import assert from 'node:assert/strict';
import test from 'node:test';
import { fromMarkdown } from 'mdast-util-from-markdown';
import { converterHtml, caminhosDeTexto } from '../js/publicacao.mjs';

const origem = 'https://oficina-exemplo.example';
const pagina = (corpo, extras = '') => `<!doctype html><html lang="pt-BR"><head><title>Guia da oficina</title><meta name="description" content="Como escolher sua peça."><link rel="canonical" href="${origem}/guias/pecas"><meta property="article:modified_time" content="2026-10-20T12:00:00Z">${extras}</head><body><nav>Menu repetido</nav><main><h1>Escolha sua peça</h1>${corpo}</main><footer>Rodapé repetido</footer></body></html>`;

test('preserva metadados editoriais e estrutura sem usar o relógio', () => {
  const html = pagina('<h2>Medidas</h2><p>Confira <strong>diâmetro</strong> e altura.</p>');
  const resultado = converterHtml(html, { url: `${origem}/guias/pecas` });
  assert.equal(resultado.indexavel, true);
  assert.equal(resultado.metadados.title, 'Guia da oficina');
  assert.equal(resultado.metadados.language, 'pt-BR');
  assert.equal(resultado.metadados.updated, '2026-10-20T12:00:00Z');
  assert.match(resultado.conteudo, /# Escolha sua peça/);
  assert.match(resultado.conteudo, /## Medidas/);
  assert.match(resultado.conteudo, /\*\*diâmetro\*\*/);
  assert.doesNotMatch(resultado.conteudo, /Menu repetido|Rodapé repetido|exported_at/);
  assert.deepEqual(resultado, converterHtml(html, { url: `${origem}/guias/pecas` }));
});

test('preserva tabelas, imagens e links absolutos sem aceitar esquemas executáveis', () => {
  const html = pagina('<p><a href="/contato">Fale conosco</a> <a href="javascript:alert(1)">Texto apenas</a></p><img src="/midia/peca.webp" alt="Peça de madeira"><table><thead><tr><th>Medida</th><th>Valor</th></tr></thead><tbody><tr><td>Altura</td><td>10 cm</td></tr></tbody></table>');
  const resultado = converterHtml(html, { url: `${origem}/guias/pecas` });
  assert.match(resultado.conteudo, /\[Fale conosco\]\(https:\/\/oficina-exemplo.example\/contato\)/);
  assert.match(resultado.conteudo, /!\[Peça de madeira\]\(https:\/\/oficina-exemplo.example\/midia\/peca.webp\)/);
  assert.match(resultado.conteudo, /\| Medida \| Valor \|/);
  assert.match(resultado.conteudo, /Texto apenas/);
  assert.doesNotMatch(resultado.conteudo, /javascript:/);
  assert.ok(resultado.links.includes(`${origem}/contato`));
});

test('texto editorial nunca vira expressão, JSX ou módulo MDX', async () => {
  const { compile } = await import('@mdx-js/mdx');
  const { default: frontmatter } = await import('remark-frontmatter');
  const html = pagina('<p>{globalThis.ataque = true}</p><p>&lt;script&gt;perigo&lt;/script&gt;</p><p>import ataque from "pacote"</p><p>export const ataque = 1</p><script>segredoDoScript()</script><form>Conteúdo do formulário<input value="privado"></form>');
  const resultado = converterHtml(html, { url: `${origem}/guias/pecas` });
  assert.doesNotMatch(resultado.conteudo, /\{globalThis|<script>|^import ataque|^export const ataque|segredoDoScript|Conteúdo do formulário/m);
  assert.match(resultado.conteudo, /&#123;globalThis/);
  const executaveis = [];
  function conferirArvore() {
    return arvore => {
      const visitar = no => {
        if (/^mdx/.test(no.type)) executaveis.push(no.type);
        for (const filho of no.children || []) visitar(filho);
      };
      visitar(arvore);
    };
  }
  await compile(resultado.conteudo, { remarkPlugins: [frontmatter, conferirArvore] });
  assert.deepEqual(executaveis, []);
});

test('noindex, rotas privadas e shell sem texto não contam como conteúdo publicável', () => {
  assert.equal(converterHtml(pagina('<p>Visível</p>', '<meta name="robots" content="noindex, follow">'), { url: `${origem}/guias/pecas` }).indexavel, false);
  assert.equal(converterHtml(pagina('<p>Dados da conta</p>'), { url: `${origem}/painel/perfil` }).indexavel, false);
  assert.equal(converterHtml(pagina('<p>Acesso da conta</p>'), { url: `${origem}/login.html` }).indexavel, false);
  assert.equal(converterHtml('<html><body><div id="root"></div><script src="/app.js"></script></body></html>', { url: origem }).indexavel, false);
});

test('nomes de metadados não dependem de caixa e citações preservam a estrutura', () => {
  const resultado = converterHtml(pagina('<blockquote>Texto citado</blockquote>', '<meta name="ROBOTS" content="NOINDEX">'), { url: `${origem}/guias/pecas` });
  assert.equal(resultado.indexavel, false);
  assert.match(resultado.conteudo, /^> Texto citado/m);
});

test('caminhos vêm da rota original mesmo quando as canônicas são iguais', () => {
  assert.deepEqual(caminhosDeTexto(origem), { md: '/index.md', mdx: '/index.mdx' });
  assert.deepEqual(caminhosDeTexto(`${origem}/guias/pecas`), { md: '/guias/pecas/index.md', mdx: '/guias/pecas/index.mdx' });
  assert.deepEqual(caminhosDeTexto(`${origem}/guias/pecas/`), caminhosDeTexto(`${origem}/guias/pecas`));
  assert.deepEqual(caminhosDeTexto(`${origem}/legado.html`), { md: '/legado.html.md', mdx: '/legado.html.mdx' });
  assert.notDeepEqual(caminhosDeTexto(`${origem}/a`), caminhosDeTexto(`${origem}/b`));
});

test('recusa origem não HTTP e separadores codificados antes de criar nomes de arquivos', () => {
  assert.throws(() => caminhosDeTexto('file:///segredo'), /endereço|URL|HTTP/i);
  assert.throws(() => caminhosDeTexto(`${origem}/segredo%2foutro`), /caminho|separador/i);
  assert.throws(() => caminhosDeTexto(`${origem}/segredo%5coutro`), /caminho|separador/i);
});

function descendentes(arvore) {
  return [arvore, ...(arvore.children || []).flatMap(descendentes)];
}

async function arvoreMdx(conteudo) {
  const { compile } = await import('@mdx-js/mdx');
  const { default: frontmatter } = await import('remark-frontmatter');
  let resultado;
  await compile(conteudo, { remarkPlugins: [frontmatter, () => arvore => { resultado = arvore; }] });
  return descendentes(resultado);
}

test('cercas editoriais e código em listas não liberam expressões ou módulos MDX', async () => {
  const casos = [
    '<pre><code>```</code></pre>',
    '<ul><li><pre><code>const exemplo = 1;</code></pre></li></ul>',
    '<blockquote><pre><code>```</code></pre></blockquote>',
  ];
  for (const prefixo of casos) {
    const resultado = converterHtml(pagina(`${prefixo}<p>{globalThis.ataque = true}</p><p>import exemplo from "pacote"</p><p>export const outro = 1</p>`), { url: origem });
    const nos = await arvoreMdx(resultado.conteudo);
    assert.deepEqual(nos.filter(no => no.type.startsWith('mdx')).map(no => no.type), [], prefixo);
    assert.ok(nos.some(no => no.type === 'code'), 'O código literal precisa continuar sendo código.');
  }
});

test('código em cercas e inline mantém seus caracteres literais nos dois formatos', async () => {
  for (const formato of ['md', 'mdx']) {
    const resultado = converterHtml(pagina('<pre><code>&lt;tag&gt;{exemplo}</code></pre><p><code>{inline} &lt;tag&gt;</code></p>'), { url: origem, formato });
    const nos = formato === 'mdx' ? await arvoreMdx(resultado.conteudo) : descendentes(fromMarkdown(resultado.conteudo));
    assert.ok(nos.some(no => no.type === 'code' && no.value === '<tag>{exemplo}'));
    assert.ok(nos.some(no => no.type === 'inlineCode' && no.value === '{inline} <tag>'));
  }
});

test('código indentado do Markdown não vira expressão executável no MDX', async () => {
  const html = '<html><head><title>Guia\n\n    {globalThis.ataque = true}</title></head><body><main><p>Conteúdo editorial.</p></main></body></html>';
  const resultado = converterHtml(html, { url: origem });
  assert.deepEqual((await arvoreMdx(resultado.conteudo)).filter(no => no.type.startsWith('mdx')), []);
});

test('Markdown conserva marcação editorial como texto e preserva código literal', () => {
  const resultado = converterHtml(pagina('<p>&lt;script&gt;globalThis.ataque = true&lt;/script&gt;</p><pre><code>&lt;script&gt;{literal}&lt;/script&gt;</code></pre>'), { url: origem, formato: 'md' });
  const nos = descendentes(fromMarkdown(resultado.conteudo));
  assert.deepEqual(nos.filter(no => no.type === 'html'), []);
  assert.ok(nos.some(no => no.type === 'text' && no.value.includes('<script>globalThis.ataque = true</script>')));
  assert.ok(nos.some(no => no.type === 'code' && no.value === '<script>{literal}</script>'));
});

test('a decisão de privacidade reconhece caracteres codificados da rota', () => {
  for (const caminho of ['/p%61inel/perfil', '/checkout%2ehtml', '/pt-BR/%63onta/index.html']) {
    const resultado = converterHtml(pagina('<p>Dados da conta.</p>'), { url: `${origem}${caminho}` });
    assert.equal(resultado.indexavel, false, caminho);
    assert.equal(resultado.motivo, 'rota_privada');
  }
  assert.equal(converterHtml(pagina('<p>Guia público.</p>'), { url: `${origem}/guias/pe%C3%A7as` }).indexavel, true);
});

test('todas as metas robots contribuem com suas restrições, inclusive none', () => {
  for (const extras of [
    '<meta name="robots" content="index, follow"><meta name="robots" content="noindex">',
    '<meta name="robots" content="none">',
    '<meta name="ROBOTS" content="index"><meta name="robots" content="NONE">',
  ]) {
    const resultado = converterHtml(pagina('<p>Conteúdo fora do índice.</p>', extras), { url: origem });
    assert.equal(resultado.indexavel, false, extras);
    assert.equal(resultado.motivo, 'noindex');
  }
});
