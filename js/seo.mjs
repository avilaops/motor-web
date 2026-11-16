import { parse } from 'parse5';
import { origemCanonica, enderecoPublico } from './descoberta-navegador.mjs';
import { serializarJsonParaScript } from './descoberta.mjs';

const nos = no => [no, ...(no.childNodes || []).flatMap(nos)];
const atributo = (no, nome) => no?.attrs?.find(item => item.name === nome)?.value;
const texto = no => no?.nodeName === '#text' ? no.value : (no?.childNodes || []).map(texto).join(' ');
const limparTexto = valor => String(valor || '').replace(/\s+/g, ' ').trim();
const ordem = (a, b) => a < b ? -1 : a > b ? 1 : 0;
const escapar = valor => String(valor).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const ROBOTS = 'index,follow,max-image-preview:large,max-snippet:-1,max-video-preview:-1';

function dataComprovada(valor) {
  if (valor === undefined || valor === null || valor === '') return undefined;
  if (typeof valor !== 'string' || !/^\d{4}-\d{2}-\d{2}(?:T[^\s]+)?$/.test(valor) || !Number.isFinite(Date.parse(valor))) throw new Error('Data de modificação inválida.');
  return new Date(valor).toISOString();
}

export function urlCanonicaDoArquivo(rota, origem) {
  const url = new URL(rota, origem);
  if (url.origin !== origem || url.search || url.hash) throw new Error('A rota precisa ser canônica da mesma origem.');
  url.pathname = url.pathname.replace(/\/index\.html?$/i, '/').replace(/\.html?$/i, '');
  return url.href;
}

function urlDeImagem(valor, base) {
  try {
    const url = new URL(valor, base);
    if (!['https:', 'http:'].includes(url.protocol) || url.username || url.password) return undefined;
    url.hash = '';
    return url.href;
  } catch { return undefined; }
}

function tipoErro(rota) {
  return /\/(?:404|500|_not-found)(?:\.html?)?(?:\/|$)/i.test(rota);
}

/** Núcleo puro: modifica somente marcação identificada pelo parser. */
export function prepararSeoHtml(html, { origem: valorOrigem, rota, nome, descricao = '', idioma = 'pt-BR', imagem, icone, modificacao, removerTagsLegadas = false }) {
  if (typeof html !== 'string' || typeof nome !== 'string' || !nome.trim()) throw new Error('HTML e nome público são obrigatórios.');
  const origem = origemCanonica(valorOrigem);
  const url = urlCanonicaDoArquivo(rota, origem);
  if (!enderecoPublico(url, origem) || tipoErro(rota)) return { html, pagina: null };
  const atualizada = dataComprovada(modificacao);
  const arvore = nos(parse(html, { sourceCodeLocationInfo: true }));
  const head = arvore.find(no => no.tagName === 'head');
  const noTitulo = arvore.find(no => no.tagName === 'title');
  const metas = arvore.filter(no => no.tagName === 'meta');
  const valorMeta = chave => atributo(metas.find(no => [atributo(no, 'name'), atributo(no, 'property')].includes(chave)), 'content');
  const titulo = limparTexto(texto(noTitulo) || texto(arvore.find(no => no.tagName === 'h1')) || nome).replace(/\s[—–|]\s/g, ' • ');
  const resumo = limparTexto(valorMeta('description') || descricao || texto(arvore.find(no => no.tagName === 'p')) || titulo);
  const imagemPrincipal = urlDeImagem(valorMeta('og:image') || imagem, url);
  if (!imagemPrincipal) throw new Error('A página pública precisa de imagem de marca válida.');
  const imagens = new Set([imagemPrincipal]);
  for (const no of arvore.filter(item => item.tagName === 'img')) {
    if (['0', '1'].includes(atributo(no, 'width')) || ['0', '1'].includes(atributo(no, 'height'))) continue;
    for (const valor of [atributo(no, 'src'), atributo(no, 'data-src')]) {
      const endereco = valor && urlDeImagem(valor, url);
      if (endereco) imagens.add(endereco);
    }
  }

  const edicoes = [], inserir = [];
  const substituir = (selecionados, tag) => {
    const existentes = selecionados.filter(no => no.sourceCodeLocation);
    if (!existentes.length) { inserir.push(tag); return; }
    existentes.forEach((no, indice) => edicoes.push({ inicio: no.sourceCodeLocation.startOffset, fim: no.sourceCodeLocation.endOffset, texto: indice ? '' : tag }));
  };
  const meta = (chave, valor, propriedade = false) => substituir(metas.filter(no => atributo(no, propriedade ? 'property' : 'name') === chave), `<meta ${propriedade ? 'property' : 'name'}="${chave}" content="${escapar(valor)}">`);
  substituir(noTitulo ? [noTitulo] : [], `<title>${escapar(titulo)}</title>`);
  meta('description', resumo);
  meta('robots', ROBOTS);
  for (const robo of ['googlebot', 'googlebot-image', 'bingbot']) {
    if (metas.some(no => atributo(no, 'name')?.toLowerCase() === robo)) meta(robo, ROBOTS);
  }
  substituir(arvore.filter(no => no.tagName === 'link' && atributo(no, 'rel')?.split(/\s+/).includes('canonical')), `<link rel="canonical" href="${escapar(url)}">`);
  for (const [chave, valor] of Object.entries({ 'og:type': valorMeta('og:type') || 'website', 'og:site_name': nome, 'og:locale': idioma.replace('-', '_'), 'og:title': titulo, 'og:description': resumo, 'og:url': url, 'og:image': imagemPrincipal, 'og:image:alt': `${nome} • ${titulo}` })) meta(chave, valor, true);
  for (const [chave, valor] of Object.entries({ 'twitter:card': 'summary_large_image', 'twitter:title': titulo, 'twitter:description': resumo, 'twitter:image': imagemPrincipal, 'twitter:image:alt': `${nome} • ${titulo}` })) meta(chave, valor);
  if (atualizada) meta('last-modified', atualizada);
  if (icone) {
    const urlIcone = urlDeImagem(icone, origem);
    if (!urlIcone) throw new Error('Ícone público inválido.');
    substituir(arvore.filter(no => no.tagName === 'link' && atributo(no, 'rel') === 'icon'), `<link rel="icon" href="${escapar(urlIcone)}">`);
  }

  const schemasExistentes = [];
  for (const no of arvore.filter(no => no.tagName === 'script' && atributo(no, 'type') === 'application/ld+json' && atributo(no, 'id') !== 'public-visibility-schema')) {
    try { const valor = JSON.parse(texto(no)); schemasExistentes.push(...(Array.isArray(valor) ? valor : valor['@graph'] || [valor])); } catch { /* O diagnóstico existente sinaliza JSON-LD manual inválido. */ }
  }
  const graph = [];
  if (!schemasExistentes.some(s => ['Organization', 'LocalBusiness', 'ProfessionalService'].includes(s['@type']))) graph.push({ '@type': 'Organization', '@id': origem + '/#organization', name: nome, url: origem + '/', image: imagemPrincipal });
  if (!schemasExistentes.some(s => s['@type'] === 'WebSite')) graph.push({ '@type': 'WebSite', '@id': origem + '/#website', name: nome, url: origem + '/', inLanguage: idioma });
  if (!schemasExistentes.some(s => ['WebPage', 'AboutPage', 'ContactPage', 'CollectionPage', 'Article'].includes(s['@type']))) graph.push({ '@type': 'WebPage', '@id': url + '#webpage', url, name: titulo, description: resumo, inLanguage: idioma, isPartOf: { '@id': origem + '/#website' }, primaryImageOfPage: { '@type': 'ImageObject', url: imagemPrincipal }, ...(atualizada ? { dateModified: atualizada } : {}) });
  if (!schemasExistentes.some(s => s['@type'] === 'BreadcrumbList')) graph.push({ '@type': 'BreadcrumbList', itemListElement: [{ '@type': 'ListItem', position: 1, name: idioma.startsWith('en') ? 'Home' : 'Início', item: origem + '/' }, ...(url === origem + '/' ? [] : [{ '@type': 'ListItem', position: 2, name: titulo, item: url }])] });
  substituir(arvore.filter(no => no.tagName === 'script' && atributo(no, 'id') === 'public-visibility-schema'), `<script id="public-visibility-schema" type="application/ld+json">${serializarJsonParaScript({ '@context': 'https://schema.org', '@graph': graph })}</script>`);

  if (removerTagsLegadas) {
    for (const no of arvore.filter(no => ['script', 'noscript'].includes(no.tagName) && no.sourceCodeLocation)) {
      const codigo = texto(no);
      if (atributo(no, 'type') === 'application/ld+json' || /self\.__next_f\.push/.test(codigo)) continue;
      if (/googletagmanager\.com\/(?:gtm|gtag|ns)|gtm\.start|google-analytics\.com/.test(codigo + ' ' + (atributo(no, 'src') || ''))) edicoes.push({ inicio: no.sourceCodeLocation.startOffset, fim: no.sourceCodeLocation.endOffset, texto: '' });
    }
  }
  if (inserir.length) {
    const fim = head?.sourceCodeLocation?.endTag?.startOffset;
    if (fim !== undefined) edicoes.push({ inicio: fim, fim, texto: inserir.join('\n') + '\n' });
    else {
      const inicio = arvore.find(no => no.tagName === 'body')?.sourceCodeLocation?.startOffset ?? html.length;
      edicoes.push({ inicio, fim: inicio, texto: `<head>${inserir.join('\n')}</head>` });
    }
  }
  edicoes.sort((a, b) => b.inicio - a.inicio);
  for (const edicao of edicoes) html = html.slice(0, edicao.inicio) + edicao.texto + html.slice(edicao.fim);
  return { html, pagina: { url, titulo, descricao: resumo, imagens: [...imagens].sort(ordem), ...(atualizada ? { modificacao: atualizada } : {}) } };
}

export function criarSitemapsPublicos({ origem: valorOrigem, paginas }) {
  const origem = origemCanonica(valorOrigem);
  const unicas = new Map();
  for (const pagina of paginas) {
    const url = new URL(pagina.url, origem);
    if (url.origin !== origem || url.search || url.hash || !enderecoPublico(url.href, origem) || tipoErro(url.pathname)) throw new Error('O sitemap aceita somente URL pública canônica da mesma origem.');
    if (/\/(?:index\.html?)$|\.html?$/i.test(url.pathname)) throw new Error('O sitemap precisa da URL canônica sem extensão HTML.');
    const anterior = unicas.get(url.href);
    const imagens = [...new Set([...(anterior?.imagens || []), ...(pagina.imagens || []).map(valor => urlDeImagem(valor, url.href)).filter(Boolean)])].sort(ordem);
    const modificacao = [anterior?.modificacao, dataComprovada(pagina.modificacao)].filter(Boolean).sort(ordem).at(-1);
    unicas.set(url.href, { url: url.href, imagens, modificacao });
  }
  const lista = [...unicas.values()].sort((a, b) => ordem(a.url, b.url));
  const abrir = '<?xml version="1.0" encoding="UTF-8"?>\n';
  const linhas = lista.map(p => `  <url><loc>${escapar(p.url)}</loc>${p.modificacao ? `<lastmod>${p.modificacao}</lastmod>` : ''}</url>`);
  const imagens = lista.filter(p => p.imagens.length).map(p => `  <url><loc>${escapar(p.url)}</loc>${p.imagens.slice(0, 1000).map(img => `<image:image><image:loc>${escapar(img)}</image:loc></image:image>`).join('')}</url>`);
  return {
    urls: `${abrir}<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${linhas.join('\n')}\n</urlset>\n`,
    imagens: `${abrir}<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:image="http://www.google.com/schemas/sitemap-image/1.1">\n${imagens.join('\n')}\n</urlset>\n`,
    indice: `${abrir}<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>${origem}/sitemap.xml</loc></sitemap><sitemap><loc>${origem}/sitemap-images.xml</loc></sitemap></sitemapindex>\n`,
    robots: ['*', 'Googlebot-Image', 'OAI-SearchBot', 'Claude-SearchBot', 'ChatGPT-User', 'Claude-User', 'GPTBot', 'ClaudeBot'].map(bot => `User-agent: ${bot}\nAllow: /\n`).join('\n') + `\nSitemap: ${origem}/sitemap-index.xml\n`,
  };
}
