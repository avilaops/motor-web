import { parse, serialize } from 'parse5';
import { createDocument } from '@mixmark-io/domino';
import TurndownService from 'turndown';
import { gfm } from 'turndown-plugin-gfm';
import { fromMarkdown } from 'mdast-util-from-markdown';

const PRIVADO = /^\/(?:[a-z]{2}(?:-[a-z]{2})?\/)?(?:api|painel|admin|dashboard|login|entrar|auth|conta|checkout|carrinho|cadastro|register)(?:\/|\.html?$|$)/i;
const REMOVER = new Set(['script', 'style', 'noscript', 'nav', 'footer', 'aside', 'form', 'input', 'button', 'select', 'textarea', 'svg', 'iframe', 'template']);

function atributo(no, nome) {
  return no?.attrs?.find(a => a.name === nome)?.value;
}

function encontrar(no, aceita) {
  if (aceita(no)) return no;
  for (const filho of no.childNodes || []) {
    const resultado = encontrar(filho, aceita);
    if (resultado) return resultado;
  }
  return undefined;
}

function todos(no, aceita, resultado = []) {
  if (aceita(no)) resultado.push(no);
  for (const filho of no.childNodes || []) todos(filho, aceita, resultado);
  return resultado;
}

function texto(no) {
  if (!no) return '';
  if (no.nodeName === '#text') return no.value;
  return (no.childNodes || []).map(texto).join(' ');
}

function endereco(valor, base, protocolos = ['http:', 'https:']) {
  try {
    const url = new URL(valor, base);
    return protocolos.includes(url.protocol) && !url.username && !url.password ? url.href : undefined;
  } catch {
    return undefined;
  }
}

function urlDaPagina(valor) {
  const url = new URL(valor);
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) {
    throw new Error('O endereço da página precisa usar HTTP ou HTTPS, sem credenciais.');
  }
  if (/%(?:2f|5c)/i.test(url.pathname) || /[\\\u0000-\u001f]/.test(decodeURIComponent(url.pathname))) {
    throw new Error('O caminho não pode conter separador codificado ou caractere de controle.');
  }
  url.search = '';
  url.hash = '';
  return url;
}

export function caminhosDeTexto(valor) {
  const url = urlDaPagina(valor);
  const caminho = url.pathname.replace(/\/+$/, '');
  const base = !caminho || caminho === '/index.html'
    ? '/index'
    : /\.html?$/i.test(caminho) ? caminho : `${caminho}/index`;
  return { md: `${base}.md`, mdx: `${base}.mdx` };
}

function limpar(no, base) {
  if (no.childNodes) {
    no.childNodes = no.childNodes.filter(filho => !REMOVER.has(filho.tagName));
    for (const filho of no.childNodes) limpar(filho, base);
  }
  if (no.attrs) {
    no.attrs = no.attrs.filter(a => !a.name.startsWith('on') && !['style', 'srcdoc'].includes(a.name));
    for (const nome of ['href', 'src']) {
      const a = no.attrs.find(item => item.name === nome);
      if (!a) continue;
      const permitido = endereco(a.value, base, nome === 'href' ? ['http:', 'https:', 'mailto:', 'tel:'] : ['http:', 'https:']);
      if (permitido) a.value = permitido;
      else no.attrs = no.attrs.filter(item => item !== a);
    }
  }
}

function escaparTexto(markdown) {
  const literais = [];
  const visitar = no => {
    const inicio = no.position?.start.offset;
    const fim = no.position?.end.offset;
    // MDX não reconhece código indentado. Só preservamos as formas literais
    // comuns aos dois formatos, delimitadas pelo parser (inclusive em listas).
    if (no.type === 'inlineCode' || (no.type === 'code' && /^(?:`{3,}|~{3,})/.test(markdown.slice(inicio, fim)))) {
      literais.push([inicio, fim]);
    } else for (const filho of no.children || []) visitar(filho);
  };
  visitar(fromMarkdown(markdown));
  const escapar = texto => texto
    .replace(/</g, '&lt;')
    .replace(/\{/g, '&#123;')
    .replace(/\}/g, '&#125;')
    .replace(/\b(import|export)\b/g, palavra => `&#${palavra.charCodeAt(0)};${palavra.slice(1)}`);
  let cursor = 0;
  let resultado = '';
  for (const [inicio, fim] of literais) {
    resultado += escapar(markdown.slice(cursor, inicio)) + markdown.slice(inicio, fim);
    cursor = fim;
  }
  return resultado + escapar(markdown.slice(cursor));
}

function valorYaml(valor) {
  return JSON.stringify(valor).replace(/[<>{}]/g, caractere => `\\u${caractere.charCodeAt(0).toString(16).padStart(4, '0')}`);
}

/** Converte o HTML do conteúdo publicado. Não lê arquivos, rede ou relógio. */
export function converterHtml(html, { url: valorUrl, formato = 'mdx' }) {
  if (typeof html !== 'string') throw new TypeError('O conteúdo HTML precisa ser texto.');
  if (!['md', 'mdx'].includes(formato)) throw new Error('Formato de texto desconhecido.');
  const url = urlDaPagina(valorUrl);
  const arvore = parse(html);
  const htmlNo = encontrar(arvore, no => no.tagName === 'html');
  const meta = chave => atributo(encontrar(arvore, no => no.tagName === 'meta' && [atributo(no, 'name'), atributo(no, 'property')].some(nome => nome?.toLowerCase() === chave)), 'content');
  const semIndice = todos(arvore, no => no.tagName === 'meta' && [atributo(no, 'name'), atributo(no, 'property')].some(nome => nome?.toLowerCase() === 'robots'))
    .some(no => /\b(?:noindex|none)\b/i.test(atributo(no, 'content') || ''));
  const canonicalNo = encontrar(arvore, no => no.tagName === 'link' && atributo(no, 'rel')?.split(/\s+/).includes('canonical'));
  const canonical = endereco(atributo(canonicalNo, 'href') || url.href, url.href) || url.href;
  const titulo = texto(encontrar(arvore, no => no.tagName === 'title')).trim();
  const conteudo = encontrar(arvore, no => no.tagName === 'main') || encontrar(arvore, no => no.tagName === 'article') || encontrar(arvore, no => no.tagName === 'body');
  limpar(conteudo, url.href);
  const textoVisivel = texto(conteudo).trim();
  const motivo = PRIVADO.test(decodeURIComponent(url.pathname)) ? 'rota_privada' : semIndice ? 'noindex' : !textoVisivel ? 'sem_conteudo' : null;
  const indexavel = motivo === null;
  const atualizado = meta('article:modified_time') || atributo(encontrar(arvore, no => no.tagName === 'time' && atributo(no, 'itemprop') === 'dateModified'), 'datetime');
  const metadados = {
    generator: '@avilaops/publicacao-texto',
    title: titulo || texto(encontrar(conteudo, no => no.tagName === 'h1')).trim(),
    ...(meta('description') ? { description: meta('description') } : {}),
    canonical,
    ...(atributo(htmlNo, 'lang') ? { language: atributo(htmlNo, 'lang') } : {}),
    ...(atualizado ? { updated: atualizado } : {}),
  };
  const conversor = new TurndownService({ headingStyle: 'atx', codeBlockStyle: 'fenced', bulletListMarker: '-' });
  conversor.use(gfm);
  const raiz = createDocument().createElement('x-turndown');
  raiz.innerHTML = serialize(conteudo);
  let corpo = conversor.turndown(raiz).trim();
  if (!encontrar(conteudo, no => no.tagName === 'h1') && metadados.title) corpo = `# ${metadados.title}\n\n${corpo}`;
  corpo = escaparTexto(corpo);
  const cabecalho = Object.entries(metadados).map(([chave, valor]) => `${chave}: ${valorYaml(valor)}`).join('\n');
  const links = [...new Set(todos(conteudo, no => no.tagName === 'a').map(no => atributo(no, 'href')).filter(Boolean))].sort();
  return { conteudo: `---\n${cabecalho}\n---\n\n${corpo}\n`, metadados, indexavel, motivo, links };
}
