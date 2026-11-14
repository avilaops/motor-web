import { parse } from 'parse5';
import { createHash } from 'node:crypto';
import { origemCanonica, enderecoPublico } from './descoberta-navegador.mjs';

const atributo = (no, nome) => no?.attrs?.find(item => item.name === nome)?.value;
const nos = no => [no, ...(no.childNodes || []).flatMap(nos)];
const texto = no => no?.nodeName === '#text' ? no.value : (no?.childNodes || []).map(texto).join(' ');
const ordem = (a, b) => a < b ? -1 : a > b ? 1 : 0;

function textoObrigatorio(valor, nome) {
  if (typeof valor !== 'string' || !valor.trim()) throw new Error(`${nome} precisa ser texto não vazio.`);
  return valor.trim();
}

export function serializarJsonParaScript(valor) {
  return JSON.stringify(valor).replace(/[<>&\u2028\u2029]/g, caractere => `\\u${caractere.charCodeAt(0).toString(16).padStart(4, '0')}`);
}

/** Gerador puro: quem chama confirma que páginas/recursos existem e são públicos. */
export function criarCatalogoPublico({ origem: valorOrigem, nome: valorNome, descricao: valorDescricao, paginas = [], recursos = [] }) {
  const origem = origemCanonica(valorOrigem);
  const nome = textoObrigatorio(valorNome, 'Nome');
  const descricao = textoObrigatorio(valorDescricao, 'Descrição');
  if (!Array.isArray(paginas) || !Array.isArray(recursos)) throw new Error('Páginas e recursos precisam ser listas.');
  const itens = [
    ...paginas.map(pagina => ({ url: pagina.url, nome: pagina.titulo, descricao: pagina.descricao, tipo: 'text/html', consultas: pagina.consultas, namespace: 'pagina' })),
    ...recursos.map(recurso => ({ ...recurso, namespace: 'recurso' })),
  ].map(item => {
    const url = enderecoPublico(item.url, origem);
    if (!url) throw new Error('O recurso precisa ter URL pública da mesma origem, sem consulta ou caminho privado.');
    const titulo = textoObrigatorio(item.nome, 'Nome do recurso');
    if (item.descricao !== undefined && typeof item.descricao !== 'string') throw new Error('Descrição do recurso precisa ser texto.');
    if (typeof item.tipo !== 'string' || !/^[a-z0-9!#$&^_.+-]+\/[a-z0-9!#$&^_.+-]+(?:\s*;[^\r\n]+)?$/i.test(item.tipo)) throw new Error('Tipo MIME do recurso inválido.');
    const consultas = item.consultas ?? [`Consultar ${titulo} em ${nome}`, `Encontrar informações públicas sobre ${titulo}`];
    if (!Array.isArray(consultas) || consultas.length < 2 || consultas.length > 5 || consultas.some(consulta => typeof consulta !== 'string' || !consulta.trim())) throw new Error('Forneça duas a cinco consultas representativas.');
    return { ...item, url, nome: titulo, descricao: item.descricao?.trim() || descricao, consultas: consultas.map(consulta => consulta.trim()) };
  }).sort((a, b) => ordem(`${a.url}\n${a.tipo}`, `${b.url}\n${b.tipo}`));
  const unicos = new Map();
  for (const item of itens) {
    const chave = `${item.url}\n${item.tipo}`;
    if (unicos.has(chave)) throw new Error('Recurso público duplicado.');
    unicos.set(chave, item);
  }
  const host = new URL(origem).hostname;
  const entries = [...unicos.values()].map(item => ({
    identifier: `urn:air:${host}:${item.namespace}:item-${createHash('sha256').update(`${item.url}\n${item.tipo}`).digest('hex').slice(0, 24)}`,
    displayName: item.nome,
    type: item.tipo,
    url: item.url,
    description: item.descricao,
    representativeQueries: item.consultas,
  }));
  const ard = { entries };
  const aiCatalogo = { specVersion: '1.0', host: { displayName: nome, identifier: host }, entries };
  return { ard, aiCatalogo, jsonArd: `${JSON.stringify(ard, null, 2)}\n`, jsonAiCatalogo: `${JSON.stringify(aiCatalogo, null, 2)}\n` };
}

export function analisarPaginaPublica(html, { url: valorUrl }) {
  if (typeof html !== 'string') throw new TypeError('HTML precisa ser texto.');
  const url = new URL(valorUrl);
  const arvore = nos(parse(html));
  const metas = arvore.filter(no => no.tagName === 'meta');
  const meta = nome => atributo(metas.find(no => [atributo(no, 'name'), atributo(no, 'property')].some(valor => valor?.toLowerCase() === nome)), 'content');
  const semIndice = metas.some(no => /^(?:robots|googlebot|bingbot)$/i.test(atributo(no, 'name') || '') && /\b(?:noindex|none)\b/i.test(atributo(no, 'content') || ''));
  const redireciona = metas.some(no => atributo(no, 'http-equiv')?.toLowerCase() === 'refresh');
  const visivel = no => !no || ['script', 'style', 'noscript', 'template', 'form', 'nav', 'footer', 'aside', 'input', 'textarea', 'select', 'button'].includes(no.tagName) ? '' : no.nodeName === '#text' ? no.value : (no.childNodes || []).map(visivel).join(' ');
  const corpo = arvore.find(no => no.tagName === 'main') || arvore.find(no => no.tagName === 'article') || arvore.find(no => no.tagName === 'body');
  const titulo = texto(arvore.find(no => no.tagName === 'title')).trim() || texto(arvore.find(no => no.tagName === 'h1')).trim();
  const schemas = [], problemas = [];
  for (const no of arvore.filter(no => no.tagName === 'script' && atributo(no, 'type')?.toLowerCase() === 'application/ld+json')) {
    try {
      const schema = JSON.parse(texto(no));
      if (schema && typeof schema === 'object') schemas.push(schema);
      else problemas.push({ codigo: 'json_ld_invalido', mensagem: 'JSON-LD precisa ser objeto ou lista.' });
    } catch { problemas.push({ codigo: 'json_ld_invalido', mensagem: 'JSON-LD não pôde ser lido como JSON.' }); }
  }
  const motivo = !enderecoPublico(url.href, url.origin) ? 'rota_privada' : semIndice ? 'noindex' : redireciona ? 'redirecionamento' : !visivel(corpo).trim() ? 'sem_conteudo' : null;
  return { publica: motivo === null, motivo, url: url.href, titulo, descricao: meta('description') || '', schemas, problemas };
}

function escaparAtributo(valor) { return valor.replace(/[&"<>]/g, caractere => ({ '&': '&amp;', '"': '&quot;', '<': '&lt;', '>': '&gt;' })[caractere]); }

/** Edita apenas tags geradas; os offsets do parser preservam o HTML original. */
export function injetarDescoberta(html, { url, catalogoUrl = '/.well-known/ard.json', moduloUrl = '/descoberta-publica.mjs', remover = false }) {
  const pagina = analisarPaginaPublica(html, { url });
  const origem = new URL(url).origin;
  if (!enderecoPublico(catalogoUrl, origem) || !enderecoPublico(moduloUrl, origem)) throw new Error('URLs de descoberta precisam ser públicas e da mesma origem.');
  const arvore = nos(parse(html, { sourceCodeLocationInfo: true }));
  const baseHtml = new URL(atributo(arvore.find(no => no.tagName === 'base' && atributo(no, 'href')), 'href') || url, url);
  const catalogoReferencia = baseHtml.origin === origem ? catalogoUrl : new URL(catalogoUrl, url).href;
  const moduloReferencia = baseHtml.origin === origem ? moduloUrl : new URL(moduloUrl, url).href;
  const gerados = arvore.filter(no => ['ard', 'contexto', 'modulo'].includes(atributo(no, 'data-descoberta')) && ['link', 'meta', 'script'].includes(no.tagName));
  const edicoes = gerados.filter(no => no.sourceCodeLocation).map(no => {
    const { startOffset, endOffset } = no.sourceCodeLocation;
    return { inicio: startOffset, fim: endOffset + (html.slice(endOffset).match(/^[\t \r\n]*/)?.[0].length || 0), texto: '' };
  });
  if (pagina.publica && !remover) {
    const manuais = arvore.filter(no => !gerados.includes(no));
    const tags = [];
    const links = manuais.filter(no => no.tagName === 'link' && atributo(no, 'rel')?.toLowerCase().split(/\s+/).includes('ard'));
    if (links.some(no => new URL(atributo(no, 'href'), baseHtml).href !== new URL(catalogoUrl, url).href)) throw new Error('Já existe um catálogo ARD manual diferente.');
    if (!links.length) tags.push(`<link rel="ard" type="application/json" href="${escaparAtributo(catalogoReferencia)}" data-descoberta="ard">`);
    if (!manuais.some(no => no.tagName === 'meta' && atributo(no, 'name') === 'descoberta-publica' && atributo(no, 'content') === 'true' && atributo(no, 'data-descoberta-publica') === 'true')) tags.push('<meta name="descoberta-publica" content="true" data-descoberta-publica="true" data-descoberta="contexto">');
    if (!manuais.some(no => no.tagName === 'script' && atributo(no, 'src') && new URL(atributo(no, 'src'), baseHtml).href === new URL(moduloUrl, url).href && atributo(no, 'type') === 'module')) tags.push(`<script type="module" src="${escaparAtributo(moduloReferencia)}" data-descoberta="modulo"></script>`);
    const head = arvore.find(no => no.tagName === 'head');
    let inicio = head?.sourceCodeLocation?.endTag?.startOffset ?? head?.sourceCodeLocation?.endOffset;
    let bloco = `${tags.join('\n')}\n`;
    if (inicio === undefined) {
      inicio = arvore.find(no => no.tagName === 'body')?.sourceCodeLocation?.startTag?.startOffset ?? arvore.find(no => no.nodeName === '#documentType')?.sourceCodeLocation?.endOffset ?? 0;
      bloco = `<head>\n${bloco}</head>\n`;
    }
    edicoes.push({ inicio, fim: inicio, texto: bloco });
  }
  edicoes.sort((a, b) => b.inicio - a.inicio);
  for (const edicao of edicoes) html = html.slice(0, edicao.inicio) + edicao.texto + html.slice(edicao.fim);
  return html;
}
