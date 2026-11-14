import assert from 'node:assert/strict';
import test from 'node:test';
import { parse } from 'parse5';

const modulo = await import('../js/descoberta.mjs').catch(() => ({}));
const origem = 'https://empresa.example';
const pagina = titulo => `<html><head><title>${titulo}</title><meta name="description" content="Conteúdo público."></head><body><main><h1>${titulo}</h1><p>Informações da empresa fictícia.</p></main></body></html>`;
const opcoes = { origem, nome: 'Empresa fictícia', descricao: 'Informações públicas.', paginas: [{ url: '/', titulo: 'Início' }, { url: '/sobre', titulo: 'Sobre' }] };

function nos(no) { return [no, ...(no.childNodes || []).flatMap(nos)]; }
function atributo(no, chave) { return no.attrs?.find(item => item.name === chave)?.value; }

test('catálogos anunciam páginas reais da mesma origem com formato ARD e compatível', () => {
  assert.equal(typeof modulo.criarCatalogoPublico, 'function');
  const resultado = modulo.criarCatalogoPublico(opcoes);
  assert.equal(resultado.ard.entries.length, 2);
  assert.deepEqual(resultado.ard.entries.map(item => item.url), ['https://empresa.example/', 'https://empresa.example/sobre']);
  for (const entrada of resultado.ard.entries) {
    assert.match(entrada.identifier, /^urn:air:empresa\.example:pagina:[a-z0-9._-]+$/);
    assert.ok(entrada.displayName);
    assert.equal(entrada.type, 'text/html');
    assert.ok(entrada.representativeQueries.length >= 2 && entrada.representativeQueries.length <= 5);
    assert.equal(Object.hasOwn(entrada, 'data'), false);
  }
  assert.equal(resultado.aiCatalogo.specVersion, '1.0');
  assert.deepEqual(resultado.aiCatalogo.entries, resultado.ard.entries);
  assert.deepEqual(JSON.parse(resultado.jsonArd), resultado.ard);
});

test('catálogo não mistura hosts e recusa origem ambígua ou recursos inexistentes privados', () => {
  assert.equal(typeof modulo.criarCatalogoPublico, 'function');
  for (const url of ['https://outra.example/sobre', '/api/dados', '/pt-BR/painel', '/checkout', '/carrinho', '/cart', '/guias/%2561pi/dados', '/guias/%2fprivado']) {
    assert.throws(() => modulo.criarCatalogoPublico({ ...opcoes, paginas: [{ url, titulo: 'Público' }] }), /públic|origem|caminho/i, url);
  }
  for (const valor of ['javascript:alert(1)', 'https://usuario:senha@empresa.example', 'https://empresa.example/base']) {
    assert.throws(() => modulo.criarCatalogoPublico({ ...opcoes, origem: valor }), /origem/i);
  }
  assert.throws(() => modulo.criarCatalogoPublico({ ...opcoes, recursos: [{ url: '/dados.json', nome: 'Dados', tipo: 'servidor inventado' }] }), /tipo|MIME/i);
});

test('ordem, identificadores e serialização são determinísticos sem alterar a entrada', () => {
  assert.equal(typeof modulo.criarCatalogoPublico, 'function');
  const entrada = structuredClone(opcoes);
  const antes = JSON.stringify(entrada);
  const a = modulo.criarCatalogoPublico(entrada);
  const b = modulo.criarCatalogoPublico({ ...entrada, paginas: [...entrada.paginas].reverse() });
  assert.equal(a.jsonArd, b.jsonArd);
  assert.equal(a.jsonAiCatalogo, b.jsonAiCatalogo);
  assert.equal(JSON.stringify(entrada), antes);
  const outro = modulo.criarCatalogoPublico({ ...entrada, origem: 'https://outra.example' });
  assert.ok(outro.ard.entries.every(item => item.url.startsWith('https://outra.example/') && item.identifier.includes('outra.example')));
});

test('metadados existentes são lidos e JSON-LD malformado não vira código executável', () => {
  assert.equal(typeof modulo.analisarPaginaPublica, 'function');
  const html = pagina('Sobre').replace('</head>', '<script type="application/ld+json">{"@type":"Organization","name":"Empresa fictícia"}</script><script type="application/ld+json">inválido</script></head>');
  const resultado = modulo.analisarPaginaPublica(html, { url: `${origem}/sobre` });
  assert.equal(resultado.publica, true);
  assert.equal(resultado.titulo, 'Sobre');
  assert.equal(resultado.schemas.length, 1);
  assert.equal(resultado.problemas[0].codigo, 'json_ld_invalido');
});

test('páginas privadas, noindex, redirects e shells não recebem contexto público', () => {
  assert.equal(typeof modulo.analisarPaginaPublica, 'function');
  const casos = [
    { html: pagina('Painel'), url: `${origem}/painel`, motivo: 'rota_privada' },
    { html: pagina('Fora').replace('</head>', '<meta name="robots" content="noindex"></head>'), url: `${origem}/fora`, motivo: 'noindex' },
    { html: pagina('Fora').replace('</head>', '<meta name="googlebot" content="none"></head>'), url: `${origem}/fora`, motivo: 'noindex' },
    { html: pagina('Redirect').replace('</head>', '<meta http-equiv="refresh" content="0;url=/sobre"></head>'), url: `${origem}/antigo`, motivo: 'redirecionamento' },
    { html: '<div id="root"></div><script src="/app.js"></script>', url: `${origem}/`, motivo: 'sem_conteudo' },
  ];
  for (const caso of casos) {
    const resultado = modulo.analisarPaginaPublica(caso.html, { url: caso.url });
    assert.equal(resultado.publica, false);
    assert.equal(resultado.motivo, caso.motivo);
  }
});

test('injeção é idempotente e preserva byte a byte JSON-LD, scripts e comentários', () => {
  assert.equal(typeof modulo.injetarDescoberta, 'function');
  const script = '<script>window.exemplo = "</head>"; window.tag = \'<link rel="ard">\';</script>';
  const jsonld = '<script type="application/ld+json">{ "@type": "Organization" }</script>';
  const original = pagina('Início').replace('</head>', `${script}${jsonld}<!-- preservar --></head>`);
  const a = modulo.injetarDescoberta(original, { url: `${origem}/` });
  assert.ok(a.includes(script) && a.includes(jsonld) && a.includes('<!-- preservar -->'));
  const arvore = nos(parse(a));
  assert.equal(arvore.filter(no => no.tagName === 'link' && atributo(no, 'rel') === 'ard').length, 1);
  assert.equal(arvore.filter(no => no.tagName === 'script' && atributo(no, 'src') === '/descoberta-publica.mjs').length, 1);
  assert.equal(arvore.filter(no => atributo(no, 'data-descoberta-publica') === 'true').length, 1);
  assert.equal(modulo.injetarDescoberta(a, { url: `${origem}/` }), a);
  assert.equal(modulo.injetarDescoberta(pagina('Painel'), { url: `${origem}/painel` }), pagina('Painel'));
});

test('serialização em script bloqueia fechamento de tag e preserva o valor JSON', () => {
  assert.equal(typeof modulo.serializarJsonParaScript, 'function');
  const valor = { nome: '</script><script>alert(1)</script>&\u2028\u2029' };
  const json = modulo.serializarJsonParaScript(valor);
  assert.doesNotMatch(json, /<|>|&|\u2028|\u2029/u);
  assert.deepEqual(JSON.parse(json), valor);
});

test('identificador de uma página existente não muda quando outra página é adicionada', () => {
  const antes = modulo.criarCatalogoPublico(opcoes);
  const depois = modulo.criarCatalogoPublico({ ...opcoes, paginas: [...opcoes.paginas, { url: '/contato', titulo: 'Contato' }] });
  assert.equal(antes.ard.entries.find(item => item.url.endsWith('/sobre')).identifier, depois.ard.entries.find(item => item.url.endsWith('/sobre')).identifier);
});

test('rotas de autenticação e escapes recursivos não entram no catálogo público', () => {
  for (const url of ['/recuperar-senha', '/configuracoes', '/guias/%2525252561pi/dados', '/pedido/token-pessoal', '/recuperar', '/redefinir', '/autorizar', '/criar']) {
    assert.throws(() => modulo.criarCatalogoPublico({ ...opcoes, paginas: [{ url, titulo: 'Não publicar' }] }), /públic|origem|caminho/i, url);
  }
});

test('compatibilidade usa a identidade host exigida pelo schema ai-catalog publicado', () => {
  assert.deepEqual(modulo.criarCatalogoPublico(opcoes).aiCatalogo.host, { displayName: 'Empresa fictícia', identifier: 'empresa.example' });
});

test('documento sem head explícito recebe tags dentro do html e mantém repetição estável', () => {
  const original = '<!doctype html><html lang="pt-BR"><body><main><h1>Empresa fictícia</h1></main></body></html>';
  const publicado = modulo.injetarDescoberta(original, { url: origem + '/' });
  assert.ok(publicado.indexOf('<head>') > publicado.indexOf('<html lang="pt-BR">'));
  assert.equal(modulo.injetarDescoberta(publicado, { url: origem + '/' }), publicado);
});

test('origem sem autoridade compatível não produz URN inválida', () => {
  assert.throws(() => modulo.criarCatalogoPublico({ ...opcoes, origem: 'https://[::1]' }), /origem|autoridade/i);
});

test('base externa não muda a origem do módulo e catálogo injetados', () => {
  const original = pagina('Empresa fictícia').replace('</head>', '<base href="https://outra.example/"></head>');
  const publicado = modulo.injetarDescoberta(original, { url: origem + '/' });
  const arvore = nos(parse(publicado));
  const script = arvore.find(no => no.tagName === 'script' && atributo(no, 'data-descoberta') === 'modulo');
  const link = arvore.find(no => no.tagName === 'link' && atributo(no, 'rel') === 'ard');
  assert.equal(new URL(atributo(script, 'src'), 'https://outra.example/').origin, origem);
  assert.equal(new URL(atributo(link, 'href'), 'https://outra.example/').origin, origem);
});

test('metatag manual incompleta não impede o marcador explícito exigido pelo adaptador', () => {
  const original = pagina('Empresa fictícia').replace('</head>', '<meta name="descoberta-publica" content="true"></head>');
  const publicado = modulo.injetarDescoberta(original, { url: origem + '/' });
  assert.equal(nos(parse(publicado)).filter(no => atributo(no, 'data-descoberta-publica') === 'true').length, 1);
});

test('descrições opcionais nunca serializam números ou objetos internos como metadados', () => {
  for (const descricao of [42, { interno: 'sentinela' }, null]) {
    assert.throws(() => modulo.criarCatalogoPublico({ ...opcoes, paginas: [{ url: '/', titulo: 'Início', descricao }] }), /descrição|texto/i);
    assert.throws(() => modulo.criarCatalogoPublico({ ...opcoes, paginas: [], recursos: [{ url: '/dados.json', nome: 'Dados', tipo: 'application/json', descricao }] }), /descrição|texto/i);
  }
  const resultado = modulo.criarCatalogoPublico({ ...opcoes, paginas: [{ url: '/', titulo: 'Início', descricao: '  ' }] });
  assert.equal(resultado.ard.entries[0].description, 'Informações públicas.');
});
