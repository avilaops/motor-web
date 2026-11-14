import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, mkdir, readFile, writeFile, unlink, access, symlink, rm, realpath } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, dirname, basename } from 'node:path';
import { parse } from 'parse5';
import { runInNewContext } from 'node:vm';
import { publicarDiretorio } from '../js/arquivos.mjs';

const origem = 'https://oficina-exemplo.example';
const html = titulo => `<html lang="pt-BR"><head><title>${titulo}</title><link rel="canonical" href="${origem}/"></head><body><main><h1>${titulo}</h1><p>Conteúdo público da oficina.</p></main></body></html>`;

async function ambiente(t) {
  const temporario = await realpath(tmpdir());
  const raiz = await mkdtemp(join(temporario, 'publicacao-texto-'));
  t.after(async () => {
    const resolvido = await realpath(raiz);
    assert.equal(dirname(resolvido), temporario);
    assert.ok(basename(resolvido).startsWith('publicacao-texto-'));
    await rm(resolvido, { recursive: true, force: true });
  });
  return raiz;
}

test('exporta todas as rotas do build e não usa canonical para nomear os arquivos', async t => {
  const raiz = await ambiente(t);
  await mkdir(join(raiz, 'guias', 'pecas'), { recursive: true });
  await writeFile(join(raiz, 'index.html'), html('Início'));
  await writeFile(join(raiz, 'guias', 'pecas', 'index.html'), html('Peças'));
  await writeFile(join(raiz, 'legado.html'), html('Legado'));
  const resultado = await publicarDiretorio({ diretorio: raiz, origem });
  assert.equal(resultado.paginas.length, 3);
  assert.match(await readFile(join(raiz, 'index.mdx'), 'utf8'), /# Início/);
  assert.match(await readFile(join(raiz, 'guias', 'pecas', 'index.md'), 'utf8'), /# Peças/);
  assert.match(await readFile(join(raiz, 'legado.html.mdx'), 'utf8'), /# Legado/);
  const manifesto = JSON.parse(await readFile(join(raiz, 'publicacao-texto.json'), 'utf8'));
  assert.equal(manifesto.paginas.length, 3);
  assert.ok(manifesto.paginas.every(p => p.md && p.mdx));
  const publicado = await readFile(join(raiz, 'guias', 'pecas', 'index.html'), 'utf8');
  assert.match(publicado, /rel="alternate"[^>]+href="\/guias\/pecas\/index.md"/);
  assert.match(await readFile(join(raiz, '_headers'), 'utf8'), /text\/markdown/);
});

test('atualização e remoção acompanham o HTML sem duplicar tags ou perder cabeçalhos', async t => {
  const raiz = await ambiente(t);
  await mkdir(join(raiz, 'guias'));
  await writeFile(join(raiz, 'index.html'), html('Início'));
  await writeFile(join(raiz, 'guias', 'index.html'), html('Antes'));
  await writeFile(join(raiz, '_headers'), '/assets/*\n  Cache-Control: public, max-age=3600\n');
  await publicarDiretorio({ diretorio: raiz, origem });
  await writeFile(join(raiz, 'guias', 'index.html'), html('Depois'));
  await publicarDiretorio({ diretorio: raiz, origem });
  assert.match(await readFile(join(raiz, 'guias', 'index.mdx'), 'utf8'), /# Depois/);
  const antes = await readFile(join(raiz, 'index.html'), 'utf8');
  await publicarDiretorio({ diretorio: raiz, origem });
  assert.equal(await readFile(join(raiz, 'index.html'), 'utf8'), antes);
  assert.equal((antes.match(/type="text\/markdown"/g) || []).length, 1);
  assert.match(await readFile(join(raiz, '_headers'), 'utf8'), /Cache-Control: public, max-age=3600/);
  await unlink(join(raiz, 'guias', 'index.html'));
  await publicarDiretorio({ diretorio: raiz, origem });
  await assert.rejects(access(join(raiz, 'guias', 'index.mdx')));
});

test('noindex, 404 e painel são excluídos e arquivos manuais são preservados', async t => {
  const raiz = await ambiente(t);
  await writeFile(join(raiz, 'index.html'), html('Início'));
  await writeFile(join(raiz, '404.html'), html('Erro'));
  await mkdir(join(raiz, 'painel'));
  await writeFile(join(raiz, 'painel', 'index.html'), html('Painel'));
  await writeFile(join(raiz, 'fora.html'), html('Fora').replace('</head>', '<meta name="robots" content="noindex"></head>'));
  await writeFile(join(raiz, 'manual.mdx'), '# Conteúdo manual\n');
  const resultado = await publicarDiretorio({ diretorio: raiz, origem });
  assert.equal(resultado.paginas.length, 1);
  assert.equal(resultado.ignoradas.length, 3);
  assert.equal(await readFile(join(raiz, 'manual.mdx'), 'utf8'), '# Conteúdo manual\n');
});

test('não sobrescreve um MDX manual que colide com a exportação', async t => {
  const raiz = await ambiente(t);
  await writeFile(join(raiz, 'index.html'), html('Início'));
  await writeFile(join(raiz, 'index.mdx'), '# Manual\n');
  await assert.rejects(publicarDiretorio({ diretorio: raiz, origem }), /existente|manual|colis/i);
  assert.equal(await readFile(join(raiz, 'index.mdx'), 'utf8'), '# Manual\n');
});

test('symlink não é seguido e shell de aplicação não simula cobertura completa', async t => {
  const raiz = await ambiente(t);
  await writeFile(join(raiz, 'index.html'), html('Início'));
  try {
    await symlink(join(raiz, 'index.html'), join(raiz, 'atalho.html'));
  } catch (erro) {
    if (!['EPERM', 'EACCES'].includes(erro.code)) throw erro;
  }
  const resultado = await publicarDiretorio({ diretorio: raiz, origem });
  assert.equal(resultado.paginas.length, 1);
  await writeFile(join(raiz, 'index.html'), '<html><body><div id="root"></div><script src="/app.js"></script></body></html>');
  await assert.rejects(publicarDiretorio({ diretorio: raiz, origem }), /conteúdo|renderiza|shell/i);
});

test('arquivos auxiliares também recusam symlink antes de ler ou gravar', async t => {
  const raiz = await ambiente(t);
  await writeFile(join(raiz, 'index.html'), html('Início'));
  await writeFile(join(raiz, 'manual.txt'), 'Preservar.');
  try { await symlink(join(raiz, 'manual.txt'), join(raiz, '_headers')); }
  catch (erro) { if (['EPERM', 'EACCES'].includes(erro.code)) { t.skip('Este Windows não autorizou criar symlink; o CI Linux executa o caso.'); return; } throw erro; }
  await assert.rejects(publicarDiretorio({ diretorio: raiz, origem }), /symlink/i);
  assert.equal(await readFile(join(raiz, 'manual.txt'), 'utf8'), 'Preservar.');
});

test('nomes literais com percentuais, Unicode e hash conservam suas rotas e diretórios', async t => {
  const raiz = await ambiente(t);
  for (const nome of ['p%61inel', 'peças #1', 'literal%2f']) {
    await mkdir(join(raiz, nome));
    await writeFile(join(raiz, nome, 'index.html'), html('Guia público'));
  }
  const resultado = await publicarDiretorio({ diretorio: raiz, origem });
  assert.deepEqual(resultado.paginas.map(p => p.rota).sort(), ['/literal%252f/', '/p%2561inel/', '/pe%C3%A7as%20%231/'].sort());
  await assert.rejects(access(join(raiz, 'painel', 'index.md')));
  for (const nome of ['p%61inel', 'peças #1', 'literal%2f']) assert.match(await readFile(join(raiz, nome, 'index.md'), 'utf8'), /Guia público/);
  assert.deepEqual((await publicarDiretorio({ diretorio: raiz, origem })).paginas, resultado.paginas);
});

test('regras noindex e none de _headers restringem a descoberta conforme URL e ordem', async t => {
  const casos = [
    { nome: 'splat global', regras: '/*\n  X-Robots-Tag: noindex\n', rotas: [] },
    { nome: 'arquivo exato', regras: '/fora.html\n  X-Robots-Tag: noindex\n', rotas: ['/'] },
    { nome: 'URL limpa', regras: '/fora\n  X-Robots-Tag: none\n', rotas: ['/'] },
    { nome: 'placeholder', regras: '/:pagina.html\n  X-Robots-Tag: noindex\n', rotas: ['/'] },
    { nome: 'host diferente', regras: 'https://preview.example/*\n  X-Robots-Tag: noindex\n', rotas: ['/fora.html', '/'] },
    { nome: 'placeholder no host', regras: 'https://:site.example/*\n  X-Robots-Tag: noindex\n', rotas: [] },
    { nome: 'remoção posterior', regras: '/*\n  X-Robots-Tag: noindex\n/fora*\n  ! X-Robots-Tag\n  X-Robots-Tag: index\n', rotas: ['/fora.html'] },
    { nome: 'restrição acumulada', regras: '/*\n  X-Robots-Tag: index\n/fora.html\n  X-Robots-Tag: noindex\n', rotas: ['/'] },
  ];
  for (const caso of casos) await t.test(caso.nome, async subt => {
    const raiz = await ambiente(subt);
    await writeFile(join(raiz, 'index.html'), html('Início'));
    await writeFile(join(raiz, 'fora.html'), html('Outra página'));
    await writeFile(join(raiz, '_headers'), caso.regras);
    const resultado = await publicarDiretorio({ diretorio: raiz, origem });
    assert.deepEqual(resultado.paginas.map(p => p.rota), caso.rotas);
    assert.ok(resultado.ignoradas.every(p => p.motivo === 'noindex'));
    const indice = await readFile(join(raiz, 'llms.txt'), 'utf8');
    if (!caso.rotas.includes('/fora.html')) assert.doesNotMatch(indice, /fora\.html\.md/);
    assert.ok((await readFile(join(raiz, '_headers'), 'utf8')).startsWith(caso.regras));
    assert.deepEqual((await publicarDiretorio({ diretorio: raiz, origem })).paginas, resultado.paginas);
  });
});

function nosHtml(no) {
  return [no, ...(no.childNodes || []).flatMap(nosHtml)];
}

test('a descoberta altera apenas elementos HTML reais e preserva strings JavaScript', async t => {
  const raiz = await ambiente(t);
  const script = '<script>window.exemplo = "</head>"; window.tag = \'<link data-publicacao-texto="md" href="/manual">\';</script>';
  await writeFile(join(raiz, 'index.html'), html('Início').replace('</head>', `${script}</head>`));
  await publicarDiretorio({ diretorio: raiz, origem });
  const publicado = await readFile(join(raiz, 'index.html'), 'utf8');
  assert.ok(publicado.includes(script));
  const nos = nosHtml(parse(publicado));
  const contexto = { window: {} };
  runInNewContext(nos.find(no => no.tagName === 'script').childNodes[0].value, contexto);
  assert.equal(contexto.window.exemplo, '</head>');
  assert.equal(contexto.window.tag, '<link data-publicacao-texto="md" href="/manual">');
  assert.equal(nos.filter(no => no.tagName === 'link' && no.attrs.some(a => a.name === 'data-publicacao-texto')).length, 3);
  await publicarDiretorio({ diretorio: raiz, origem });
  assert.equal(await readFile(join(raiz, 'index.html'), 'utf8'), publicado);
});

test('IDs 404 e 500 em rotas públicas não são confundidos com páginas de erro da raiz', async t => {
  const raiz = await ambiente(t);
  for (const caminho of ['produtos/404', 'guias/500']) {
    await mkdir(join(raiz, caminho), { recursive: true });
    await writeFile(join(raiz, caminho, 'index.html'), html('Guia público'));
  }
  for (const nome of ['404.html', '500.html', '_not-found.html']) await writeFile(join(raiz, nome), html('Erro'));
  const resultado = await publicarDiretorio({ diretorio: raiz, origem });
  assert.deepEqual(resultado.paginas.map(p => p.rota), ['/guias/500/', '/produtos/404/']);
  assert.equal(resultado.ignoradas.filter(p => p.motivo === 'pagina_de_erro').length, 3);
  assert.match(await readFile(join(raiz, 'produtos/404/index.md'), 'utf8'), /Guia público/);
});

test('uma página excluída perde arquivos e links gerados, preservando links manuais', async t => {
  const raiz = await ambiente(t);
  const manual = '<link rel="alternate" type="application/rss+xml" href="/feed.xml">';
  await writeFile(join(raiz, 'index.html'), html('Início').replace('</head>', `${manual}</head>`));
  await publicarDiretorio({ diretorio: raiz, origem });
  const publicado = await readFile(join(raiz, 'index.html'), 'utf8');
  await writeFile(join(raiz, 'index.html'), publicado.replace('</head>', '<meta name="robots" content="noindex"></head>'));
  const resultado = await publicarDiretorio({ diretorio: raiz, origem });
  assert.equal(resultado.paginas.length, 0);
  const excluido = await readFile(join(raiz, 'index.html'), 'utf8');
  assert.doesNotMatch(excluido, /data-publicacao-texto/);
  assert.ok(excluido.includes(manual));
  await assert.rejects(access(join(raiz, 'index.md')));
  await assert.rejects(access(join(raiz, 'index.mdx')));
  assert.doesNotMatch(await readFile(join(raiz, 'llms.txt'), 'utf8'), /\]\([^\n]*index\.md\)/);
  await publicarDiretorio({ diretorio: raiz, origem });
  assert.equal(await readFile(join(raiz, 'index.html'), 'utf8'), excluido);
});
