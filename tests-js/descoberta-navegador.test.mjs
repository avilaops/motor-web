import assert from 'node:assert/strict';
import test from 'node:test';
import { createWindow } from '@mixmark-io/domino';
import { readFile } from 'node:fs/promises';
import { runInNewContext } from 'node:vm';
import * as modulo from '../js/descoberta-navegador.mjs';

const origem = 'https://empresa.example';
const marca = '<meta name="descoberta-publica" content="true" data-descoberta-publica="true">';
function ambiente(marcado = true) {
  const janela = createWindow(`<html><head><title>Empresa fictícia</title><meta name="description" content="Informações públicas.">${marcado ? marca : ''}<script type="application/ld+json">{"@type":"Organization","name":"Empresa fictícia"}</script><script type="application/ld+json">inválido</script></head><body><nav><a href="/sobre">Sobre</a><a href="/servicos">Serviços</a><a href="/servicos">Repetido</a><a href="https://outra.example/sobre">Externo</a><a href="/api/dados">API</a><a href="/painel">Painel</a><a href="/checkout">Comprar</a><a href="/carrinho">Carrinho</a><a href="/sobre?token=segredo">Token</a><a href="/segredo" hidden>Segredo</a><a href="/interno" data-descoberta-privada>Interno</a><a href="javascript:alert(1)">JS</a></nav><main><form><input name="senha" value="nao-exportar"></form><script>window.sessao='nao-exportar';</script><p>Texto privado que não integra metadados.</p></main></body></html>`, origem);
  const ferramentas = new Map();
  const api = { async registerTool(ferramenta, opcoes) {
    assert.ok(opcoes.signal instanceof AbortSignal);
    if (ferramentas.has(ferramenta.name)) throw new Error('Duplicada');
    ferramentas.set(ferramenta.name, ferramenta);
    opcoes.signal.addEventListener('abort', () => ferramentas.delete(ferramenta.name), { once: true });
  } };
  return { documento: janela.document, localizacao: { href: origem + '/' }, controlador: new AbortController(), modelContext: api, ferramentas };
}

test('API atual registra apenas duas operações de leitura e retorna metadados sem formulário ou sessão', async () => {
  assert.equal(typeof modulo.instalarFerramentasPublicas, 'function');
  const contexto = ambiente();
  contexto.documento.modelContext = contexto.modelContext;
  const resultado = await modulo.instalarFerramentasPublicas({ ...contexto, modelContext: undefined });
  assert.equal(resultado.ativo, true);
  assert.deepEqual([...contexto.ferramentas.keys()].sort(), ['consultar_pagina_publica', 'localizar_links_publicos']);
  const ferramenta = contexto.ferramentas.get('consultar_pagina_publica');
  assert.equal(ferramenta.annotations.readOnlyHint, true);
  const dados = await ferramenta.execute({});
  assert.equal(dados.titulo, 'Empresa fictícia');
  assert.equal(dados.schemas.length, 1);
  assert.equal(dados.problemas[0].codigo, 'json_ld_invalido');
  assert.doesNotMatch(JSON.stringify(dados), /nao-exportar|Texto privado|senha|sessao/);
});

test('navegação só devolve links públicos da mesma origem e valida consulta/limite', async () => {
  assert.equal(typeof modulo.instalarFerramentasPublicas, 'function');
  const contexto = ambiente();
  await modulo.instalarFerramentasPublicas(contexto);
  const busca = contexto.ferramentas.get('localizar_links_publicos');
  const dados = await busca.execute({ consulta: 's', limite: 10 });
  assert.deepEqual(dados.links.map(link => link.url), ['https://empresa.example/servicos', 'https://empresa.example/sobre']);
  assert.equal((await busca.execute({ consulta: 's', limite: 1 })).links.length, 1);
  for (const entrada of [{ consulta: '' }, { consulta: 1 }, { consulta: 's', limite: 100 }, []]) assert.equal((await busca.execute(entrada)).erro.codigo, 'entrada_invalida');
});

test('API anterior usa o mesmo adaptador e duplicação não repete registros', async () => {
  assert.equal(typeof modulo.instalarFerramentasPublicas, 'function');
  const contexto = ambiente();
  const opcoes = { ...contexto, modelContext: undefined, navegador: { modelContext: contexto.modelContext } };
  const a = await modulo.instalarFerramentasPublicas(opcoes);
  const b = await modulo.instalarFerramentasPublicas(opcoes);
  assert.equal(a, b);
  assert.equal(contexto.ferramentas.size, 2);
  a.desinstalar();
  assert.equal(contexto.ferramentas.size, 0);
});

test('ausência de API, marcador, noindex ou rota pública não instala ferramentas', async () => {
  assert.equal(typeof modulo.instalarFerramentasPublicas, 'function');
  const casos = [
    { ...ambiente(), modelContext: undefined },
    ambiente(false),
    { ...ambiente(), localizacao: { href: origem + '/painel' } },
  ];
  const semIndice = ambiente();
  semIndice.documento.head.insertAdjacentHTML('beforeend', '<meta name="robots" content="noindex">');
  casos.push(semIndice);
  for (const contexto of casos) {
    assert.equal((await modulo.instalarFerramentasPublicas(contexto)).ativo, false);
    assert.equal(contexto.ferramentas.size, 0);
  }
});

test('abort, pagehide e mudança para contexto privado encerram a disponibilidade', async () => {
  assert.equal(typeof modulo.instalarFerramentasPublicas, 'function');
  const contexto = ambiente();
  await modulo.instalarFerramentasPublicas(contexto);
  const consulta = contexto.ferramentas.get('consultar_pagina_publica');
  contexto.localizacao.href = origem + '/conta';
  assert.equal((await consulta.execute({})).erro.codigo, 'contexto_nao_publico');
  contexto.controlador.abort();
  assert.equal(contexto.ferramentas.size, 0);
  const outro = ambiente();
  await modulo.instalarFerramentasPublicas(outro);
  const evento = outro.documento.createEvent('Event'); evento.initEvent('pagehide', false, false);
  outro.documento.defaultView.dispatchEvent(evento);
  assert.equal(outro.ferramentas.size, 0);
});

test('falha no segundo registro aborta o primeiro e execução cancelada não lê DOM', async () => {
  assert.equal(typeof modulo.instalarFerramentasPublicas, 'function');
  const contexto = ambiente();
  const registrar = contexto.modelContext.registerTool;
  contexto.modelContext.registerTool = async (ferramenta, opcoes) => {
    if (ferramenta.name === 'localizar_links_publicos') throw new Error('Indisponível');
    return registrar(ferramenta, opcoes);
  };
  assert.equal((await modulo.instalarFerramentasPublicas(contexto)).ativo, false);
  assert.equal(contexto.ferramentas.size, 0);
  const outro = ambiente();
  await modulo.instalarFerramentasPublicas(outro);
  const controlador = new AbortController(); controlador.abort();
  assert.equal((await outro.ferramentas.get('consultar_pagina_publica').execute({}, { signal: controlador.signal })).erro.codigo, 'execucao_cancelada');
});

test('bootstrap sem API mantém importação dinâmica e rede desativadas', async () => {
  let codigo;
  try { codigo = await readFile(new URL('../js/descoberta-bootstrap.mjs', import.meta.url), 'utf8'); } catch { codigo = ''; }
  assert.ok(codigo, 'O bootstrap precisa existir.');
  const contexto = ambiente();
  let importacoes = 0;
  runInNewContext(codigo, { document: contexto.documento, navigator: {}, location: contexto.localizacao, AbortController, fetch() { throw new Error('Rede proibida.'); } }, { importModuleDynamically() { importacoes++; throw new Error('Importação inesperada.'); } });
  assert.equal(importacoes, 0);
});

test('links relativos respeitam a URL do documento e base externa não atravessa origem', async () => {
  const contexto = ambiente();
  const janela = createWindow(`<head>${marca}</head><body><nav><a href="detalhe">Detalhe</a></nav></body>`, origem + '/guias/');
  contexto.documento = janela.document;
  contexto.localizacao.href = origem + '/guias/';
  await modulo.instalarFerramentasPublicas(contexto);
  const ferramenta = contexto.ferramentas.get('localizar_links_publicos');
  assert.deepEqual((await ferramenta.execute({ consulta: 'detalhe' })).links.map(link => link.url), ['https://empresa.example/guias/detalhe']);
  contexto.documento.head.insertAdjacentHTML('afterbegin', '<base href="https://outra.example/">');
  assert.equal((await ferramenta.execute({ consulta: 'detalhe' })).links.length, 0);
});

test('cancelamento durante registro assíncrono não retorna instalação ativa', async () => {
  const contexto = ambiente();
  const registrar = contexto.modelContext.registerTool;
  let liberar;
  const espera = new Promise(resolve => { liberar = resolve; });
  contexto.modelContext.registerTool = async (ferramenta, opcoes) => {
    await registrar(ferramenta, opcoes);
    if (ferramenta.name === 'localizar_links_publicos') await espera;
  };
  const instalacao = modulo.instalarFerramentasPublicas(contexto);
  await new Promise(resolve => setImmediate(resolve));
  contexto.controlador.abort(); liberar();
  assert.equal((await instalacao).ativo, false);
  assert.equal(contexto.ferramentas.size, 0);
});

test('pedido com token e rotas de autorização não são expostos como navegação pública', async () => {
  const contexto = ambiente();
  contexto.documento.querySelector('nav').innerHTML = '<a href="/pedido/token-pessoal">Pedido pessoal</a><a href="/recuperar">Pedido de recuperação</a><a href="/redefinir">Pedido de redefinição</a><a href="/autorizar">Pedido de autorização</a><a href="/criar">Pedido de conta</a><a href="/blog/pedido-de-orcamento">Pedido de orçamento público</a>';
  await modulo.instalarFerramentasPublicas(contexto);
  const ferramenta = contexto.ferramentas.get('localizar_links_publicos');
  assert.deepEqual((await ferramenta.execute({ consulta: 'pedido' })).links.map(link => link.url), ['https://empresa.example/blog/pedido-de-orcamento']);
});
