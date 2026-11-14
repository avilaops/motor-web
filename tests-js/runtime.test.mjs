import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { converterHtml } from '../js/publicacao.mjs';

test('bundle de Worker converte sem DOM global e produz os mesmos bytes do Node', async () => {
  const bundle = await build({
    entryPoints: [fileURLToPath(new URL('../js/publicacao.mjs', import.meta.url))],
    bundle: true,
    platform: 'neutral',
    mainFields: ['module', 'main'],
    // Reproduz a variante de Turndown escolhida no runtime de Worker.
    // O restante continua usando as condições do publicador de servidor.
    alias: { turndown: fileURLToPath(new URL('../node_modules/turndown/lib/turndown.browser.es.js', import.meta.url)) },
    format: 'iife',
    globalName: 'publicacao',
    write: false,
    logLevel: 'silent',
  });
  const contexto = vm.createContext({ TextEncoder, TextDecoder, URL, setTimeout, clearTimeout, setInterval, clearInterval }, {
    codeGeneration: { strings: false, wasm: false },
  });
  new vm.Script(bundle.outputFiles[0].text).runInContext(contexto);
  assert.equal(vm.runInContext('typeof document', contexto), 'undefined');
  const html = '<html lang="pt-BR"><head><title>Oficina exemplo</title></head><body><main><h1>Oficina exemplo</h1><p>Conteúdo com {expressão} e <a href="/servicos/">serviços</a>.</p><table><tr><th>Item</th><th>Prazo</th></tr><tr><td>Revisão</td><td>Agendado</td></tr></table></main></body></html>';
  for (const formato of ['md', 'mdx']) {
    const opcoes = { url: 'https://oficina-exemplo.example/', formato };
    const esperado = converterHtml(html, opcoes);
    const resultado = contexto.publicacao.converterHtml(html, opcoes);
    assert.equal(resultado.conteudo, esperado.conteudo);
    assert.equal(resultado.indexavel, esperado.indexavel);
  }
});
