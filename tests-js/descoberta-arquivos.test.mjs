import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, mkdir, readFile, writeFile, realpath, rm, symlink, access } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, basename, join } from 'node:path';
import { spawnSync } from 'node:child_process';

const modulo = await import('../js/descoberta-arquivos.mjs').catch(() => ({}));
const origem = 'https://empresa.example';
const html = titulo => `<html><head><title>${titulo}</title><meta name="description" content="Informações públicas."></head><body><main><h1>${titulo}</h1><p>Conteúdo da empresa fictícia.</p></main></body></html>`;
async function ambiente(t) {
  const temporario = await realpath(tmpdir());
  const raiz = await mkdtemp(join(temporario, 'descoberta-publica-'));
  t.after(async () => {
    const resolvido = await realpath(raiz);
    assert.equal(dirname(resolvido), temporario);
    assert.ok(basename(resolvido).startsWith('descoberta-publica-'));
    await rm(resolvido, { recursive: true, force: true });
  });
  return raiz;
}

test('publicador emite catálogos e módulos, preservando redirects, robots e HTML privado', async t => {
  assert.equal(typeof modulo.publicarDescoberta, 'function');
  const raiz = await ambiente(t);
  await mkdir(join(raiz, 'sobre'));
  await mkdir(join(raiz, 'painel'));
  await writeFile(join(raiz, 'index.html'), html('Empresa fictícia'));
  await writeFile(join(raiz, 'sobre/index.html'), html('Sobre'));
  await writeFile(join(raiz, 'painel/index.html'), html('Painel'));
  await writeFile(join(raiz, 'fora.html'), html('Fora'));
  await writeFile(join(raiz, '_headers'), '/fora\n  X-Robots-Tag: noindex\n');
  await writeFile(join(raiz, '_redirects'), '/antigo /sobre 301\n');
  await writeFile(join(raiz, 'robots.txt'), 'User-agent: *\nDisallow: /painel\n');
  const resultado = await modulo.publicarDescoberta({ diretorio: raiz, origem });
  assert.deepEqual(resultado.paginas.map(pagina => pagina.url), ['https://empresa.example/', 'https://empresa.example/sobre/']);
  const ard = JSON.parse(await readFile(join(raiz, '.well-known/ard.json'), 'utf8'));
  assert.equal(ard.entries.length, 2);
  assert.equal(JSON.parse(await readFile(join(raiz, 'ai-catalog.json'), 'utf8')).specVersion, '1.0');
  assert.equal(await readFile(join(raiz, 'ai-catalog.json'), 'utf8'), await readFile(join(raiz, '.well-known/ai-catalog.json'), 'utf8'));
  assert.match(await readFile(join(raiz, 'descoberta-publica.mjs'), 'utf8'), /import\(/);
  assert.match(await readFile(join(raiz, 'descoberta-navegador.mjs'), 'utf8'), /instalarFerramentasPublicas/);
  assert.match(await readFile(join(raiz, 'index.html'), 'utf8'), /data-descoberta-publica="true"/);
  assert.equal(await readFile(join(raiz, 'painel/index.html'), 'utf8'), html('Painel'));
  assert.equal(await readFile(join(raiz, 'fora.html'), 'utf8'), html('Fora'));
  assert.equal(await readFile(join(raiz, '_redirects'), 'utf8'), '/antigo /sobre 301\n');
  assert.equal(await readFile(join(raiz, 'robots.txt'), 'utf8'), 'User-agent: *\nDisallow: /painel\n');
  const antes = await readFile(join(raiz, 'index.html'), 'utf8');
  const arquivoAntes = await readFile(join(raiz, '.well-known/ard.json'), 'utf8');
  await modulo.publicarDescoberta({ diretorio: raiz, origem });
  assert.equal(await readFile(join(raiz, 'index.html'), 'utf8'), antes);
  assert.equal(await readFile(join(raiz, '.well-known/ard.json'), 'utf8'), arquivoAntes);
});

test('prefixos excluem catálogo e injeção sem confundir nomes de outros segmentos', async t => {
  assert.equal(typeof modulo.publicarDescoberta, 'function');
  const raiz = await ambiente(t);
  for (const caminho of ['core/produto', 'loja/produto', 'lojamento']) {
    await mkdir(join(raiz, caminho), { recursive: true });
    await writeFile(join(raiz, caminho, 'index.html'), html('Produto fictício'));
  }
  await writeFile(join(raiz, 'index.html'), html('Empresa fictícia'));
  const resultado = await modulo.publicarDescoberta({ diretorio: raiz, origem, excluirPrefixos: ['/core/', '/loja/'] });
  assert.deepEqual(resultado.paginas.map(pagina => pagina.url), ['https://empresa.example/', 'https://empresa.example/lojamento/']);
  for (const caminho of ['core/produto', 'loja/produto']) assert.equal(await readFile(join(raiz, caminho, 'index.html'), 'utf8'), html('Produto fictício'));
  assert.equal(resultado.ignoradas.filter(item => item.motivo === 'prefixo_excluido').length, 2);
});

test('prefixos malformados e escapes são recusados antes de qualquer escrita', async t => {
  assert.equal(typeof modulo.publicarDescoberta, 'function');
  const raiz = await ambiente(t);
  await writeFile(join(raiz, 'index.html'), html('Empresa fictícia'));
  for (const prefixo of ['../fora', '//outra.example', '/a/../painel', '/%2f', '/a\\b', '/algo?q=1']) {
    await assert.rejects(modulo.publicarDescoberta({ diretorio: raiz, origem, excluirPrefixos: [prefixo] }), /prefixo|caminho/i);
  }
  assert.equal(await readFile(join(raiz, 'index.html'), 'utf8'), html('Empresa fictícia'));
  await assert.rejects(access(join(raiz, '.well-known/ard.json')));
});

test('arquivos manuais não são sobrescritos e colisão não deixa HTML parcialmente alterado', async t => {
  assert.equal(typeof modulo.publicarDescoberta, 'function');
  const raiz = await ambiente(t);
  await writeFile(join(raiz, 'index.html'), html('Empresa fictícia'));
  await writeFile(join(raiz, 'ai-catalog.json'), '{"manual":true}\n');
  await assert.rejects(modulo.publicarDescoberta({ diretorio: raiz, origem }), /manual|existente|colis/i);
  assert.equal(await readFile(join(raiz, 'index.html'), 'utf8'), html('Empresa fictícia'));
  assert.equal(await readFile(join(raiz, 'ai-catalog.json'), 'utf8'), '{"manual":true}\n');
});

test('destino symlink não permite escrever fora da raiz publicada', async t => {
  assert.equal(typeof modulo.publicarDescoberta, 'function');
  const raiz = await ambiente(t), fora = await ambiente(t);
  await writeFile(join(raiz, 'index.html'), html('Empresa fictícia'));
  try { await symlink(fora, join(raiz, '.well-known'), 'junction'); }
  catch (erro) { if (['EPERM', 'EACCES'].includes(erro.code)) { t.skip('Symlink exige permissão neste Windows.'); return; } throw erro; }
  await assert.rejects(modulo.publicarDescoberta({ diretorio: raiz, origem }), /symlink|diretório|caminho/i);
  await assert.rejects(access(join(fora, 'ard.json')));
});

test('CLI aceita --excluir-prefixo repetido e publica somente páginas vigentes', async t => {
  const raiz = await ambiente(t);
  for (const caminho of ['core', 'loja']) {
    await mkdir(join(raiz, caminho));
    await writeFile(join(raiz, caminho, 'index.html'), html('Não publicar'));
  }
  await writeFile(join(raiz, 'index.html'), html('Empresa fictícia'));
  const resultado = spawnSync(process.execPath, ['bin/publicar-descoberta.mjs', '--diretorio', raiz, '--origem', origem, '--excluir-prefixo', '/core/', '--excluir-prefixo', '/loja/'], { cwd: new URL('..', import.meta.url), encoding: 'utf8' });
  assert.equal(resultado.status, 0, resultado.stderr);
  assert.equal(JSON.parse(resultado.stdout).paginas, 1);
  assert.equal(JSON.parse(await readFile(join(raiz, '.well-known/ard.json'), 'utf8')).entries.length, 1);
});

test('página que se torna noindex perde somente a integração gerada', async t => {
  const raiz = await ambiente(t);
  await writeFile(join(raiz, 'index.html'), html('Empresa fictícia'));
  await modulo.publicarDescoberta({ diretorio: raiz, origem });
  const alterado = (await readFile(join(raiz, 'index.html'), 'utf8')).replace('</head>', '<meta name="robots" content="noindex"></head>');
  await writeFile(join(raiz, 'index.html'), alterado);
  const resultado = await modulo.publicarDescoberta({ diretorio: raiz, origem });
  assert.equal(resultado.paginas.length, 0);
  const publicado = await readFile(join(raiz, 'index.html'), 'utf8');
  assert.doesNotMatch(publicado, /data-descoberta=|data-descoberta-publica=/);
  assert.match(publicado, /name="robots" content="noindex"/);
});
