import { readdir, readFile, writeFile, mkdir, lstat, realpath, unlink } from 'node:fs/promises';
import { resolve, relative, dirname, sep } from 'node:path';
import { parse } from 'parse5';
import { converterHtml, caminhosDeTexto } from './publicacao.mjs';

// Compartilhados pelos adaptadores de arquivos; não fazem parte do contrato tipado.
export { destinoSeguro, listarHtml, rotaDoArquivo, regrasRobots, noindexNosCabecalhos };

const GERADOR = '@avilaops/publicacao-texto';
const MARCA = `generator: "${GERADOR}"`;
const MANIFESTO = 'publicacao-texto.json';

async function lerSeExiste(arquivo) {
  try { return await readFile(arquivo, 'utf8'); }
  catch (erro) { if (erro.code === 'ENOENT') return undefined; throw erro; }
}

function dentro(raiz, arquivo) {
  const diferenca = relative(raiz, arquivo);
  if (diferenca === '..' || diferenca.startsWith(`..${sep}`) || resolve(raiz, diferenca) !== arquivo) {
    throw new Error('O caminho de saída escapou do diretório publicado.');
  }
}

async function destinoSeguro(raiz, caminho) {
  if (typeof caminho !== 'string' || !caminho.startsWith('/') || caminho.includes('\\')) throw new Error('Caminho de saída inválido.');
  const partes = caminho.slice(1).split('/').map(parte => decodeURIComponent(parte));
  if (partes.some(parte => !parte || ['.', '..'].includes(parte) || /[\\/\u0000-\u001f]/.test(parte))) throw new Error('Caminho de saída inválido.');
  const arquivo = resolve(raiz, ...partes);
  dentro(raiz, arquivo);
  let pai = dirname(arquivo);
  while (pai !== raiz) {
    try {
      const info = await lstat(pai);
      if (info.isSymbolicLink()) throw new Error('Um diretório de saída é um symlink.');
      dentro(raiz, await realpath(pai));
    } catch (erro) { if (erro.code !== 'ENOENT') throw erro; }
    pai = dirname(pai);
  }
  try { if ((await lstat(arquivo)).isSymbolicLink()) throw new Error('Um arquivo de saída é um symlink.'); }
  catch (erro) { if (erro.code !== 'ENOENT') throw erro; }
  return arquivo;
}

async function listarHtml(raiz, pasta = raiz, encontrados = [], ignorados = []) {
  const entradas = await readdir(pasta, { withFileTypes: true });
  entradas.sort((a, b) => a.name.localeCompare(b.name, 'en'));
  for (const entrada of entradas) {
    const arquivo = resolve(pasta, entrada.name);
    if (entrada.isSymbolicLink()) { ignorados.push({ arquivo: relative(raiz, arquivo).split(sep).join('/'), motivo: 'symlink' }); continue; }
    if (entrada.isDirectory()) {
      if (!entrada.name.startsWith('.') && !['node_modules', '_next', '_astro'].includes(entrada.name)) await listarHtml(raiz, arquivo, encontrados, ignorados);
    } else if (/\.html?$/i.test(entrada.name)) encontrados.push(arquivo);
  }
  return { encontrados, ignorados };
}

function rotaDoArquivo(raiz, arquivo) {
  const relativo = relative(raiz, arquivo).split(sep).map(encodeURIComponent).join('/');
  const rota = `/${relativo}`.replace(/(?:^|\/)index\.html?$/i, '/');
  return rota.replace(/\/{2,}/g, '/');
}

function secaoGerada(texto, conteudo, nome) {
  const inicio = `<!-- publicacao-texto:${nome}:inicio -->`;
  const fim = `<!-- publicacao-texto:${nome}:fim -->`;
  const bloco = `${inicio}\n${conteudo.trim()}\n${fim}`;
  const primeiro = texto.indexOf(inicio);
  const ultimo = primeiro >= 0 ? texto.indexOf(fim, primeiro) : -1;
  if (primeiro >= 0 && ultimo >= 0) return `${texto.slice(0, primeiro)}${bloco}${texto.slice(ultimo + fim.length)}`.trimEnd() + '\n';
  return `${texto.trimEnd()}\n\n${bloco}\n`.trimStart();
}

function padraoDeCabecalho(padrao) {
  const compilar = (valor, separador) => new RegExp('^' + (valor.match(/\*|:[A-Za-z]\w*|[^*:]+|./g) || []).map(parte => {
    if (parte === '*') return '.*';
    if (/^:[A-Za-z]\w*$/.test(parte)) return separador;
    return parte.replace(/[\\^$.*+?()[\]{}|]/g, '\\$&');
  }).join('') + '$');
  const absoluto = padrao.match(/^https:\/\/([^/]+)(\/.*)?$/);
  if (absoluto) {
    const host = compilar(absoluto[1].toLowerCase(), '[^./]+');
    const caminho = compilar(absoluto[2] || '/', '[^/]+');
    return url => host.test(url.hostname.toLowerCase()) && caminho.test(url.pathname);
  }
  const caminho = compilar(padrao, '[^/]+');
  return url => caminho.test(url.pathname);
}

function regrasRobots(cabecalhos) {
  const regras = [];
  let regra;
  for (const linha of cabecalhos.split(/\r?\n/)) {
    const valor = linha.trim();
    if (!valor || valor.startsWith('#')) continue;
    if (!/^\s/.test(linha)) {
      regra = { corresponde: padraoDeCabecalho(valor), diretivas: [] };
      regras.push(regra);
    } else if (regra) {
      if (/^!\s+x-robots-tag\s*$/i.test(valor)) regra.diretivas.push(null);
      else {
        const cabecalho = valor.match(/^x-robots-tag\s*:\s*(.*)$/i);
        if (cabecalho) regra.diretivas.push(cabecalho[1]);
      }
    }
  }
  return regras;
}

function noindexNosCabecalhos(regras, endereco) {
  const url = new URL(endereco);
  const enderecos = [url];
  // Pages também serve arquivos .html em URLs sem essa extensão.
  if (/\.html?$/i.test(url.pathname)) {
    const limpo = new URL(url);
    limpo.pathname = limpo.pathname.replace(/\.html?$/i, '');
    enderecos.push(limpo);
  }
  return enderecos.some(atual => {
    let valores = [];
    for (const regra of regras) {
      if (!regra.corresponde(atual)) continue;
      for (const diretiva of regra.diretivas) {
        if (diretiva === null) valores = [];
        else valores.push(diretiva);
      }
    }
    return valores.some(valor => /\b(?:noindex|none)\b/i.test(valor));
  });
}

function referencias(html, caminhos) {
  const nos = [];
  const visitar = no => { nos.push(no); for (const filho of no.childNodes || []) visitar(filho); };
  visitar(parse(html, { sourceCodeLocationInfo: true }));
  const atributo = (no, nome) => no.attrs?.find(a => a.name === nome)?.value;
  const links = nos.filter(no => no.tagName === 'link');
  const gerados = links.filter(no => ['md', 'mdx', 'llms'].includes(atributo(no, 'data-publicacao-texto')));
  const edicoes = gerados.filter(no => no.sourceCodeLocation).map(no => {
    const { startOffset, endOffset } = no.sourceCodeLocation;
    const fim = endOffset + (html.slice(endOffset).match(/^[\t \r\n]*/)?.[0].length || 0);
    return { inicio: startOffset, fim, texto: '' };
  });
  if (caminhos) {
    const tags = [
      `<link rel="alternate" type="text/markdown" href="${caminhos.md}" data-publicacao-texto="md">`,
      `<link rel="alternate" type="text/mdx" href="${caminhos.mdx}" data-publicacao-texto="mdx">`,
    ];
    if (!links.some(no => !gerados.includes(no) && atributo(no, 'rel')?.toLowerCase().split(/\s+/).includes('describedby'))) tags.push('<link rel="describedby" href="/llms.txt" data-publicacao-texto="llms">');
    const head = nos.find(no => no.tagName === 'head');
    const local = head.sourceCodeLocation;
    let inicio = local?.endTag?.startOffset ?? local?.endOffset;
    if (inicio === undefined) inicio = head.childNodes?.map(no => no.sourceCodeLocation?.endOffset).filter(valor => valor !== undefined).at(-1);
    let bloco = `${tags.join('\n')}\n`;
    if (inicio === undefined) {
      inicio = nos.find(no => no.tagName === 'html')?.sourceCodeLocation?.startTag?.endOffset
        ?? nos.find(no => no.tagName === 'body')?.sourceCodeLocation?.startTag?.startOffset
        ?? nos.find(no => no.nodeName === '#documentType')?.sourceCodeLocation?.endOffset ?? 0;
      bloco = `<head>\n${bloco}</head>\n`;
    }
    edicoes.push({ inicio, fim: inicio, texto: bloco });
  }
  // As posições vêm do parser HTML. Scripts e comentários ficam byte a byte
  // intactos; a árvore inteira não é reserializada.
  edicoes.sort((a, b) => b.inicio - a.inicio);
  for (const edicao of edicoes) html = html.slice(0, edicao.inicio) + edicao.texto + html.slice(edicao.fim);
  return html;
}

/** Executado no artefato de build, antes de qualquer publicação. */
export async function publicarDiretorio({ diretorio, origem }) {
  caminhosDeTexto(origem);
  const raiz = await realpath(resolve(diretorio));
  const manifestoArquivo = await destinoSeguro(raiz, `/${MANIFESTO}`);
  const llmsArquivo = await destinoSeguro(raiz, '/llms.txt');
  const cabecalhosArquivo = await destinoSeguro(raiz, '/_headers');
  const anteriorTexto = await lerSeExiste(manifestoArquivo);
  const anterior = anteriorTexto ? JSON.parse(anteriorTexto) : null;
  if (anterior && (anterior.gerador !== GERADOR || !Array.isArray(anterior.paginas))) throw new Error('Já existe um manifesto manual com o mesmo nome.');
  const anteriores = new Set((anterior?.paginas || []).flatMap(pagina => [pagina.md, pagina.mdx]));
  const cabecalhos = await lerSeExiste(cabecalhosArquivo) || '';
  const restricoes = regrasRobots(cabecalhos);
  const { encontrados, ignorados } = await listarHtml(raiz);
  const ignoradas = [...ignorados];
  const paginas = [];
  const saidas = new Map();
  const htmlAtualizados = new Map();
  for (const arquivo of encontrados) {
    const rota = rotaDoArquivo(raiz, arquivo);
    const html = await readFile(arquivo, 'utf8');
    htmlAtualizados.set(arquivo, referencias(html));
    if (/^(?:404|500|_not-found)(?:\.html?|\/index\.html?)$/i.test(relative(raiz, arquivo).split(sep).join('/'))) { ignoradas.push({ rota, motivo: 'pagina_de_erro' }); continue; }
    const url = new URL(rota, origem).href;
    if (noindexNosCabecalhos(restricoes, url)) { ignoradas.push({ rota, motivo: 'noindex' }); continue; }
    const mdx = converterHtml(html, { url, formato: 'mdx' });
    if (!mdx.indexavel) {
      if (mdx.motivo === 'sem_conteudo') throw new Error(`A rota ${rota} não tem conteúdo renderizado; um shell de aplicação não comprova cobertura.`);
      ignoradas.push({ rota, motivo: mdx.motivo });
      continue;
    }
    const caminhos = caminhosDeTexto(url);
    htmlAtualizados.set(arquivo, referencias(html, caminhos));
    const md = converterHtml(html, { url, formato: 'md' });
    for (const [tipo, resultado] of [['md', md], ['mdx', mdx]]) {
      const destino = await destinoSeguro(raiz, caminhos[tipo]);
      if (saidas.has(destino)) throw new Error(`Colisão entre rotas em ${caminhos[tipo]}.`);
      const existente = await lerSeExiste(destino);
      if (existente !== undefined && (!anteriores.has(caminhos[tipo]) || !existente.includes(MARCA))) throw new Error(`O arquivo existente ${caminhos[tipo]} é manual e não será sobrescrito.`);
      saidas.set(destino, resultado.conteudo);
    }
    paginas.push({ rota, url, ...caminhos, title: mdx.metadados.title, ...(mdx.metadados.updated ? { updated: mdx.metadados.updated } : {}), arquivo, html });
  }
  const manifestadas = paginas.map(({ arquivo, html, ...pagina }) => pagina);
  const atuais = new Set(manifestadas.flatMap(pagina => [pagina.md, pagina.mdx]));
  // Valida todos os destinos antes de gravar e só remove saídas que o manifesto
  // anterior reconhece e que ainda contêm a marca deste gerador.
  const remover = [];
  for (const caminho of anteriores) {
    if (atuais.has(caminho)) continue;
    if (!/\.mdx?$/.test(caminho)) throw new Error('O manifesto anterior contém um destino inválido.');
    const destino = await destinoSeguro(raiz, caminho);
    const texto = await lerSeExiste(destino);
    if (texto !== undefined && texto.includes(MARCA)) remover.push(destino);
  }
  for (const [arquivo, conteudo] of saidas) {
    await mkdir(dirname(arquivo), { recursive: true });
    await writeFile(arquivo, conteudo, 'utf8');
  }
  const llms = await lerSeExiste(llmsArquivo);
  const indice = manifestadas.map(p => `- [${p.title.replace(/[\r\n\[\]]/g, ' ')}](${new URL(p.md, origem).href})`).join('\n');
  await writeFile(llmsArquivo, secaoGerada(llms || `# ${manifestadas.find(p => p.rota === '/')?.title || 'Conteúdo público'}\n`, `## Versões de texto das páginas\n\n${indice}`, 'indice'), 'utf8');
  for (const [arquivo, html] of htmlAtualizados) await writeFile(arquivo, html, 'utf8');
  const inicio = '# publicacao-texto:inicio';
  const fim = '# publicacao-texto:fim';
  const bloco = `${inicio}\n/*.md\n  ! Content-Type\n  Content-Type: text/markdown; charset=utf-8\n  X-Robots-Tag: noindex\n/*.mdx\n  ! Content-Type\n  Content-Type: text/mdx; charset=utf-8\n  X-Robots-Tag: noindex\n${fim}`;
  const primeiro = cabecalhos.indexOf(inicio);
  const ultimo = primeiro >= 0 ? cabecalhos.indexOf(fim, primeiro) : -1;
  const atualizados = primeiro >= 0 && ultimo >= 0 ? `${cabecalhos.slice(0, primeiro)}${bloco}${cabecalhos.slice(ultimo + fim.length)}` : `${cabecalhos.trimEnd()}\n\n${bloco}\n`.trimStart();
  await writeFile(cabecalhosArquivo, atualizados, 'utf8');
  for (const arquivo of remover) await unlink(arquivo);
  const manifesto = { gerador: GERADOR, versao: 1, paginas: manifestadas, ignoradas };
  await writeFile(manifestoArquivo, `${JSON.stringify(manifesto, null, 2)}\n`, 'utf8');
  return manifesto;
}
