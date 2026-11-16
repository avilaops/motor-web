import { readFile, writeFile, realpath, access } from 'node:fs/promises';
import { resolve, relative, sep } from 'node:path';
import { execFileSync } from 'node:child_process';
import { destinoSeguro, listarHtml, rotaDoArquivo } from './arquivos.mjs';
import { prepararSeoHtml, criarSitemapsPublicos, urlCanonicaDoArquivo } from './seo.mjs';
import { analisarPaginaPublica } from './descoberta.mjs';
import { origemCanonica } from './descoberta-navegador.mjs';

async function lerSeExiste(arquivo) {
  try { return await readFile(arquivo, 'utf8'); } catch (erro) { if (erro.code === 'ENOENT') return undefined; throw erro; }
}

function ultimaModificacao({ configuracao, rota, relativo, repositorio }) {
  if (configuracao.modificacoes?.[rota]) return configuracao.modificacoes[rota];
  if (!repositorio) return undefined;
  const arquivos = [...(configuracao.fontesGlobais || []), ...(configuracao.fontesPorRota?.[rota] || [relativo])];
  if (arquivos.some(p => typeof p !== 'string' || p.startsWith('-') || p.split(/[\\/]/).includes('..'))) throw new Error('Fonte Git inválida.');
  if (!arquivos.length) return undefined;
  try { return execFileSync('git', ['-C', repositorio, 'log', '-1', '--format=%cI', '--', ...arquivos], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim() || undefined; }
  catch { return undefined; }
}

/** Adaptador de build: I/O e histórico ficam fora do núcleo determinístico. */
export async function publicarSeo({ diretorio, configuracao, repositorio }) {
  const raiz = await realpath(resolve(diretorio));
  const origem = origemCanonica(configuracao.origem);
  for (const recurso of [configuracao.imagem, configuracao.icone].filter(Boolean)) {
    const url = new URL(recurso, origem);
    if (url.origin === origem) {
      try { await access(await destinoSeguro(raiz, url.pathname)); }
      catch { throw new Error('Imagem ou ícone de marca local ausente: ' + url.pathname); }
    }
  }
  const excluir = (configuracao.excluirRotas || []).map(rota => new URL(urlCanonicaDoArquivo(rota, origem)).pathname.replace(/\/$/, ''));
  const { encontrados } = await listarHtml(raiz);
  const paginas = [], planos = [], ignoradas = [];
  for (const arquivo of encontrados) {
    const rota = rotaDoArquivo(raiz, arquivo);
    const canonica = new URL(urlCanonicaDoArquivo(rota, origem)).pathname;
    if (excluir.some(prefixo => canonica.replace(/\/$/, '') === prefixo || canonica.startsWith(prefixo + '/'))) { ignoradas.push({ rota, motivo: 'exclusao_explicita' }); continue; }
    const html = await readFile(arquivo, 'utf8');
    const relativo = relative(raiz, arquivo).split(sep).join('/');
    const modificacao = ultimaModificacao({ configuracao, rota, relativo, repositorio });
    const resultado = prepararSeoHtml(html, { ...configuracao, origem, rota, modificacao });
    if (!resultado.pagina) { ignoradas.push({ rota, motivo: 'rota_nao_publica' }); continue; }
    const diagnostico = analisarPaginaPublica(resultado.html, { url: resultado.pagina.url });
    if (!diagnostico.publica || diagnostico.problemas.length) throw new Error('HTML público inválido em ' + rota + ': ' + (diagnostico.motivo || diagnostico.problemas[0].codigo));
    paginas.push(resultado.pagina);
    planos.push({ arquivo, conteudo: resultado.html });
  }
  if (!paginas.length) throw new Error('O artefato não tem páginas públicas.');
  const sitemaps = criarSitemapsPublicos({ origem, paginas });
  const arquivos = { '/sitemap.xml': sitemaps.urls, '/sitemap-images.xml': sitemaps.imagens, '/sitemap-index.xml': sitemaps.indice, '/robots.txt': sitemaps.robots };
  const headersArquivo = await destinoSeguro(raiz, '/_headers');
  const headers = await lerSeExiste(headersArquivo) || '';
  const inicio = '# seo-publico:inicio', fim = '# seo-publico:fim';
  const rotas = [...new Set(paginas.map(p => new URL(p.url).pathname))].sort();
  const bloco = [inicio, ...rotas.flatMap(rota => [rota, '  ! X-Robots-Tag', '  X-Robots-Tag: index,follow,max-image-preview:large,max-snippet:-1,max-video-preview:-1']), fim].join('\n');
  const primeiro = headers.indexOf(inicio), ultimo = headers.indexOf(fim, primeiro);
  if (primeiro >= 0 && ultimo < 0) throw new Error('Bloco de cabeçalhos SEO incompleto.');
  arquivos['/_headers'] = primeiro >= 0 ? headers.slice(0, primeiro) + bloco + headers.slice(ultimo + fim.length) : `${headers.trimEnd()}\n\n${bloco}\n`.trimStart();
  const erroArquivo = await destinoSeguro(raiz, '/404.html');
  if (!await lerSeExiste(erroArquivo)) arquivos['/404.html'] = '<!doctype html><html lang="' + (configuracao.idioma || 'pt-BR') + '"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width"><meta name="robots" content="noindex"><title>404</title></head><body><main><h1>404</h1><p>Esta página não está disponível.</p><a href="/">Início</a></main></body></html>\n';
  arquivos['/seo-publicacao.json'] = JSON.stringify({ gerador: '@avilaops/publicacao-texto/seo', origem, paginas, ignoradas }, null, 2) + '\n';
  for (const [caminho, conteudo] of Object.entries(arquivos)) planos.push({ arquivo: await destinoSeguro(raiz, caminho), conteudo });
  for (const plano of planos) await writeFile(plano.arquivo, plano.conteudo, 'utf8');
  return { paginas, ignoradas };
}
