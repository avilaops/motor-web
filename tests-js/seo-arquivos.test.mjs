import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, mkdir, readFile, writeFile, rm, realpath } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, dirname, basename } from 'node:path';

const modulo = await import('../js/seo-arquivos.mjs').catch(() => ({}));
const origem = 'https://oficina.example';
const html = '<html><head><title>Oficina exemplo</title><meta name="robots" content="noindex"></head><body><main><h1>Informações da oficina</h1><p>Serviços e informações públicas.</p></main></body></html>';

async function ambiente(t) {
  const temporario = await realpath(tmpdir());
  const raiz = await mkdtemp(join(temporario, 'seo-publico-'));
  t.after(async () => { assert.equal(dirname(await realpath(raiz)), temporario); assert.ok(basename(raiz).startsWith('seo-publico-')); await rm(raiz, { recursive: true, force: true }); });
  await writeFile(join(raiz, 'index.html'), html);
  await writeFile(join(raiz, 'marca.svg'), '<svg xmlns="http://www.w3.org/2000/svg" width="120" height="60"><rect width="120" height="60"/></svg>');
  return raiz;
}

test('processa o artefato público, gera dois sitemaps e conserva exclusões explícitas', async t => {
  assert.equal(typeof modulo.publicarSeo, 'function');
  const raiz = await ambiente(t);
  await writeFile(join(raiz, 'contato.html'), html.replace('Oficina exemplo', 'Contato público'));
  await writeFile(join(raiz, 'interno.html'), html);
  const resultado = await modulo.publicarSeo({ diretorio: raiz, configuracao: { origem, nome: 'Oficina exemplo', imagem: '/marca.svg', excluirRotas: ['/interno'], modificacoes: { '/': '2026-10-02T13:20:00Z' } } });
  assert.equal(resultado.paginas.length, 2);
  assert.ok(resultado.paginas.some(p => p.url === origem + '/contato'));
  assert.match(await readFile(join(raiz, 'sitemap.xml'), 'utf8'), /<loc>https:\/\/oficina.example\/contato<\/loc>/);
  assert.match(await readFile(join(raiz, 'sitemap-images.xml'), 'utf8'), /image:loc/);
  assert.match(await readFile(join(raiz, 'sitemap-index.xml'), 'utf8'), /sitemap-images.xml/);
  assert.match(await readFile(join(raiz, 'robots.txt'), 'utf8'), /Allow: \//);
  assert.match(await readFile(join(raiz, 'interno.html'), 'utf8'), /noindex/);
  assert.match(await readFile(join(raiz, '404.html'), 'utf8'), /noindex/);
  assert.match(await readFile(join(raiz, '_headers'), 'utf8'), /\/contato\n  ! X-Robots-Tag/);
});

test('falha antes de publicar quando a imagem de marca local está ausente', async t => {
  assert.equal(typeof modulo.publicarSeo, 'function');
  const raiz = await ambiente(t);
  await assert.rejects(modulo.publicarSeo({ diretorio: raiz, configuracao: { origem, nome: 'Oficina exemplo', imagem: '/inexistente.png' } }), /imagem|Imagem/);
});
