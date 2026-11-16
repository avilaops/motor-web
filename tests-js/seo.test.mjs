import assert from 'node:assert/strict';
import test from 'node:test';
import { parse } from 'parse5';

const modulo = await import('../js/seo.mjs').catch(() => ({}));
const origem = 'https://oficina.example';
const opcoes = { origem, nome: 'Oficina exemplo', descricao: 'Serviços e informações da oficina fictícia.', idioma: 'pt-BR', imagem: '/marca.png', icone: '/favicon.svg' };
const html = '<!doctype html><html lang="pt-BR"><head><title>Peças &amp; serviços</title><meta name="robots" content="noindex,nofollow"><link rel="canonical" href="https://antiga.example/"></head><body><main><h1>Peças e serviços</h1><p>Descrição pública de peças e serviços.</p><img src="/pecas.jpg" alt="Peças"><img src="javascript:alert(1)"></main><script>const original = "<meta name=robots>";</script></body></html>';
const nos = no => [no, ...(no.childNodes || []).flatMap(nos)];
const atributo = (no, nome) => no?.attrs?.find(item => item.name === nome)?.value;

test('prepara metadados públicos, canonical e imagens sem alterar o corpo ou scripts', () => {
  assert.equal(typeof modulo.prepararSeoHtml, 'function');
  const resultado = modulo.prepararSeoHtml(html, { ...opcoes, rota: '/pecas.html', modificacao: '2026-10-02T13:20:00Z' });
  assert.equal(resultado.pagina.url, origem + '/pecas');
  assert.equal(resultado.pagina.modificacao, '2026-10-02T13:20:00.000Z');
  assert.deepEqual(resultado.pagina.imagens, [origem + '/marca.png', origem + '/pecas.jpg']);
  assert.ok(resultado.html.includes(html.slice(html.indexOf('<body>'))));
  const arvore = nos(parse(resultado.html));
  const meta = nome => atributo(arvore.find(no => no.tagName === 'meta' && [atributo(no, 'name'), atributo(no, 'property')].includes(nome)), 'content');
  assert.equal(meta('robots'), 'index,follow,max-image-preview:large,max-snippet:-1,max-video-preview:-1');
  assert.equal(meta('og:url'), origem + '/pecas');
  assert.equal(meta('og:image'), origem + '/marca.png');
  assert.equal(meta('twitter:card'), 'summary_large_image');
  assert.equal(arvore.filter(no => no.tagName === 'link' && atributo(no, 'rel') === 'canonical').length, 1);
  const ld = arvore.find(no => no.tagName === 'script' && atributo(no, 'id') === 'public-visibility-schema');
  const schema = JSON.parse(ld.childNodes[0].value);
  assert.equal(schema['@graph'].find(item => item['@type'] === 'WebPage').url, origem + '/pecas');
});

test('saída é determinística, idempotente e escapa conteúdo dentro de JSON e atributos', () => {
  assert.equal(typeof modulo.prepararSeoHtml, 'function');
  const config = { ...opcoes, nome: 'Oficina </script><script>alert(1)</script>', rota: '/' };
  const a = modulo.prepararSeoHtml(html, config);
  const b = modulo.prepararSeoHtml(a.html, config);
  assert.equal(a.html, b.html);
  assert.doesNotMatch(a.html, /"name":"Oficina <\/script>/);
  assert.match(a.html, /\\u003c\/script/);
  assert.equal(a.pagina.modificacao, undefined);
});

test('não torna rotas privadas nem páginas de erro indexáveis e rejeita data inválida', () => {
  assert.equal(typeof modulo.prepararSeoHtml, 'function');
  for (const rota of ['/admin/', '/carrinho.html', '/checkout/', '/404.html', '/pt-BR/painel/', '/guias/%2561pi/dados']) {
    const resultado = modulo.prepararSeoHtml(html, { ...opcoes, rota });
    assert.equal(resultado.pagina, null, rota);
    assert.match(resultado.html, /noindex/);
  }
  assert.throws(() => modulo.prepararSeoHtml(html, { ...opcoes, rota: '/', modificacao: 'ontem' }), /data/i);
});

test('sitemaps contêm apenas canonicals públicos únicos, imagens e datas comprovadas', () => {
  assert.equal(typeof modulo.criarSitemapsPublicos, 'function');
  const paginas = [
    { url: origem + '/', imagens: [origem + '/marca.png'] },
    { url: origem + '/pecas', imagens: [origem + '/pecas?a=1&b=2'], modificacao: '2026-10-02T13:20:00.000Z' },
    { url: origem + '/', imagens: [origem + '/marca.png'] },
  ];
  const resultado = modulo.criarSitemapsPublicos({ origem, paginas });
  assert.equal((resultado.urls.match(/<url>/g) || []).length, 2);
  assert.equal((resultado.urls.match(/<lastmod>/g) || []).length, 1);
  assert.match(resultado.imagens, /xmlns:image="http:\/\/www.google.com\/schemas\/sitemap-image\/1.1"/);
  assert.match(resultado.imagens, /a=1&amp;b=2/);
  assert.match(resultado.indice, /sitemap-images.xml/);
  assert.match(resultado.robots, /User-agent: Googlebot-Image\nAllow: \//);
  assert.match(resultado.robots, /User-agent: OAI-SearchBot\nAllow: \//);
  assert.match(resultado.robots, /User-agent: Claude-SearchBot\nAllow: \//);
  for (const url of ['https://www.oficina.example/', origem + '/admin/', origem + '/?sessao=1', origem + '/#sobre']) {
    assert.throws(() => modulo.criarSitemapsPublicos({ origem, paginas: [{ url, imagens: [] }] }), /públic|canônic|origem/i);
  }
});
