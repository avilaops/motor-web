import { readFile, writeFile, mkdir, realpath } from 'node:fs/promises';
import { resolve, dirname, relative, sep } from 'node:path';
import { createHash } from 'node:crypto';
import { destinoSeguro, listarHtml, rotaDoArquivo, regrasRobots, noindexNosCabecalhos } from './arquivos.mjs';
import { criarCatalogoPublico, analisarPaginaPublica, injetarDescoberta } from './descoberta.mjs';
import { origemCanonica } from './descoberta-navegador.mjs';

const GERADOR = '@avilaops/publicacao-texto/descoberta';
const MANIFESTO = '/descoberta-publicacao.json';
const DESTINOS = ['/.well-known/ard.json', '/ai-catalog.json', '/.well-known/ai-catalog.json', '/descoberta-publica.mjs', '/descoberta-navegador.mjs'];
const hash = texto => createHash('sha256').update(texto).digest('hex');

async function lerSeExiste(arquivo) {
  try { return await readFile(arquivo, 'utf8'); }
  catch (erro) { if (erro.code === 'ENOENT') return undefined; throw erro; }
}

function normalizarPrefixos(prefixos) {
  if (!Array.isArray(prefixos)) throw new Error('Prefixos precisam ser uma lista.');
  return [...new Set(prefixos.map(prefixo => {
    if (typeof prefixo !== 'string' || !prefixo.startsWith('/') || prefixo.startsWith('//') || /[\\?#\u0000-\u001f]/.test(prefixo)) throw new Error('Prefixo absoluto inválido.');
    let caminho = prefixo;
    try {
      for (let vez = 0; vez < 3; vez++) {
        if (/%(?:2f|5c)/i.test(caminho)) throw new Error();
        const decodificado = decodeURIComponent(caminho);
        if (decodificado === caminho) break;
        caminho = decodificado;
      }
    } catch { throw new Error('Prefixo contém escape de caminho inválido.'); }
    if (caminho.split('/').some(parte => ['.', '..'].includes(parte)) || /[\\\u0000-\u001f]/.test(caminho)) throw new Error('Prefixo contém escape de caminho inválido.');
    return caminho.replace(/\/+$/, '') || '/';
  }))].sort();
}

function excluido(rota, prefixos) {
  let caminho;
  try { caminho = decodeURIComponent(rota).replace(/\/+$/, '') || '/'; } catch { return true; }
  return prefixos.some(prefixo => prefixo === '/' || caminho === prefixo || caminho.startsWith(prefixo + '/'));
}

function cabecalhosAtualizados(texto) {
  const inicio = '# descoberta-publica:inicio', fim = '# descoberta-publica:fim';
  const bloco = [inicio, ...DESTINOS.flatMap(caminho => [caminho, '  ! Content-Type', `  Content-Type: ${caminho.endsWith('.json') ? 'application/json' : 'text/javascript'}; charset=utf-8`]), fim].join('\n');
  const primeiro = texto.indexOf(inicio), ultimo = primeiro >= 0 ? texto.indexOf(fim, primeiro) : -1;
  if (primeiro >= 0 && ultimo < 0) throw new Error('Bloco de cabeçalhos de descoberta incompleto.');
  return primeiro >= 0 ? texto.slice(0, primeiro) + bloco + texto.slice(ultimo + fim.length) : `${texto.trimEnd()}\n\n${bloco}\n`.trimStart();
}

/** Adaptador de build. Todos os destinos/colisões são validados antes de gravar. */
export async function publicarDescoberta({ diretorio, origem: valorOrigem, nome, descricao, excluirPrefixos = [] }) {
  const origem = origemCanonica(valorOrigem);
  const prefixos = normalizarPrefixos(excluirPrefixos);
  const raiz = await realpath(resolve(diretorio));
  const manifestoArquivo = await destinoSeguro(raiz, MANIFESTO);
  const headersArquivo = await destinoSeguro(raiz, '/_headers');
  const anterioresTexto = await lerSeExiste(manifestoArquivo);
  const anterior = anterioresTexto ? JSON.parse(anterioresTexto) : null;
  if (anterior && (anterior.gerador !== GERADOR || anterior.origem !== origem || !Array.isArray(anterior.arquivos) || anterior.arquivos.some(item => !DESTINOS.includes(item.caminho) || typeof item.sha256 !== 'string'))) throw new Error('Manifesto existente manual ou de outra origem.');
  const cabecalhos = await lerSeExiste(headersArquivo) || '';
  const restricoes = regrasRobots(cabecalhos);
  const encontrados = await listarHtml(raiz);
  const ignoradas = [...encontrados.ignorados];
  const paginas = [], problemas = [], htmlAtualizados = new Map();
  for (const arquivo of encontrados.encontrados) {
    const rota = rotaDoArquivo(raiz, arquivo);
    if (excluido(rota, prefixos)) { ignoradas.push({ rota, motivo: 'prefixo_excluido' }); continue; }
    const html = await readFile(arquivo, 'utf8');
    const url = new URL(rota, origem).href;
    const erro = /^(?:404|500|_not-found)(?:\.html?|\/index\.html?)$/i.test(relative(raiz, arquivo).split(sep).join('/'));
    const pagina = analisarPaginaPublica(html, { url });
    const motivo = erro ? 'pagina_de_erro' : noindexNosCabecalhos(restricoes, url) ? 'noindex' : pagina.motivo;
    if (motivo) {
      ignoradas.push({ rota, motivo });
      htmlAtualizados.set(arquivo, injetarDescoberta(html, { url, remover: true }));
      continue;
    }
    paginas.push({ rota, url, titulo: pagina.titulo || rota, descricao: pagina.descricao, schemas: pagina.schemas });
    problemas.push(...pagina.problemas.map(problema => ({ rota, ...problema })));
    htmlAtualizados.set(arquivo, injetarDescoberta(html, { url }));
  }
  paginas.sort((a, b) => a.url < b.url ? -1 : a.url > b.url ? 1 : 0);
  const principal = paginas.find(pagina => pagina.rota === '/') || paginas[0];
  const catalogos = criarCatalogoPublico({ origem, nome: nome || principal?.titulo || new URL(origem).hostname, descricao: descricao || principal?.descricao || 'Páginas públicas deste site.', paginas });
  const saidas = new Map([
    ['/.well-known/ard.json', catalogos.jsonArd],
    ['/ai-catalog.json', catalogos.jsonAiCatalogo],
    ['/.well-known/ai-catalog.json', catalogos.jsonAiCatalogo],
    ['/descoberta-publica.mjs', await readFile(new URL('./descoberta-bootstrap.mjs', import.meta.url), 'utf8')],
    ['/descoberta-navegador.mjs', await readFile(new URL('./descoberta-navegador.mjs', import.meta.url), 'utf8')],
  ]);
  const arquivos = [], planos = [];
  for (const [caminho, conteudo] of saidas) {
    const destino = await destinoSeguro(raiz, caminho);
    const existente = await lerSeExiste(destino);
    const registrado = anterior?.arquivos.find(item => item.caminho === caminho);
    if (existente !== undefined && (!registrado || registrado.sha256 !== hash(existente))) throw new Error(`O arquivo existente ${caminho} é manual ou foi editado e não será sobrescrito.`);
    arquivos.push({ caminho, sha256: hash(conteudo) });
    planos.push([destino, conteudo]);
  }
  const manifesto = { gerador: GERADOR, versao: 1, origem, paginas, ignoradas, problemas, arquivos };
  planos.push([headersArquivo, cabecalhosAtualizados(cabecalhos)], ...htmlAtualizados, [manifestoArquivo, `${JSON.stringify(manifesto, null, 2)}\n`]);
  for (const [destino, conteudo] of planos) {
    await mkdir(dirname(destino), { recursive: true });
    await writeFile(destino, conteudo, 'utf8');
  }
  return manifesto;
}
