// Estas regras também são usadas pelo gerador. Sem DOM, rede ou estado global.
export function origemCanonica(valor) {
  let url;
  try { url = new URL(valor); } catch { throw new Error('Origem HTTP(S) inválida.'); }
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.pathname !== '/' || url.search || url.hash) {
    throw new Error('A origem precisa ser HTTP(S), sem credenciais, caminho ou consulta.');
  }
  if (!/^[a-z0-9.-]+$/i.test(url.hostname)) throw new Error('A origem precisa ter uma autoridade de domínio compatível.');
  return url.origin;
}

export function enderecoPublico(valor, origem) {
  try {
    const base = origemCanonica(origem);
    if (typeof valor !== 'string' || !valor.trim() || /[\\\u0000-\u001f]/.test(valor)) return undefined;
    const url = new URL(valor, base);
    if (url.origin !== base || !['http:', 'https:'].includes(url.protocol) || url.username || url.password) return undefined;
    let caminho = url.pathname;
    for (let vez = 0; vez < 3; vez++) {
      if (/%(?:2f|5c)/i.test(caminho) || /[\\\u0000-\u001f]/.test(caminho)) return undefined;
      const decodificado = decodeURIComponent(caminho);
      if (decodificado === caminho) break;
      caminho = decodificado;
    }
    if (/%[0-9a-f]{2}/i.test(caminho)) return undefined;
    if (/(?:^|\/)(?:api|admin|painel|dashboard|login|logout|entrar|sair|auth|oauth|autorizar|conta|account|checkout|carrinho|cart|cadastro|criar|register|signin|signup|session|password|senha|recuperar|redefinir|recuperar-senha|redefinir-senha|reset-password|meus-pedidos|pedido|pedidos|orders|billing|financeiro|settings|configuracoes|private|privado|_next|_astro)(?:\/|\.html?$|$)/i.test(caminho)) return undefined;
    if (url.search) return undefined;
    url.hash = '';
    return url.href;
  } catch { return undefined; }
}

function contextoPublico(documento, localizacao) {
  try {
    const url = new URL(localizacao.href);
    const marca = documento.head?.querySelector('meta[name="descoberta-publica"][content="true"][data-descoberta-publica="true"]');
    const noindex = [...documento.querySelectorAll('meta[name]')].some(meta => /^(?:robots|googlebot|bingbot)$/i.test(meta.getAttribute('name')) && /\b(?:noindex|none)\b/i.test(meta.getAttribute('content') || ''));
    return Boolean(marca && !noindex && enderecoPublico(url.href, url.origin));
  } catch { return false; }
}

function textoPublico(no) {
  if (!no) return '';
  if (no.nodeType === 3) return no.nodeValue;
  if (no.nodeType === 1 && (['SCRIPT', 'STYLE', 'FORM', 'INPUT', 'TEXTAREA', 'SELECT', 'BUTTON'].includes(no.tagName) || no.hasAttribute('hidden') || no.hasAttribute('data-descoberta-privada') || no.getAttribute('aria-hidden') === 'true')) return '';
  return [...(no.childNodes || [])].map(textoPublico).join(' ');
}

const falha = codigo => ({ erro: { codigo } });

/** Adaptador com dependências explícitas; não lê storage, formulário ou rede. */
export async function instalarFerramentasPublicas({ documento, modelContext, navegador, localizacao, controlador }) {
  const inativo = motivo => ({ ativo: false, motivo, nomes: [], desinstalar() {} });
  const api = modelContext ?? documento?.modelContext ?? navegador?.modelContext;
  if (!api || typeof api.registerTool !== 'function') return inativo('api_ausente');
  if (!controlador?.signal || controlador.signal.aborted || !contextoPublico(documento, localizacao)) return inativo('contexto_nao_publico');
  // O estado pertence ao Document injetado, nunca a um registro global do módulo.
  if (documento.__descobertaPublica) return documento.__descobertaPublica;
  const nomes = ['consultar_pagina_publica', 'localizar_links_publicos'];
  const registrados = [];
  let encerrado = false;
  const janela = documento.defaultView;
  let promessa;
  const limpar = () => {
    if (encerrado) return;
    encerrado = true;
    if (!controlador.signal.aborted) controlador.abort();
    // Implementações anteriores podem ignorar o segundo argumento de registro.
    if (typeof api.unregisterTool === 'function') for (const nome of registrados) {
      try { api.unregisterTool(nome); } catch { /* A ferramenta já pode ter sido retirada pelo sinal. */ }
    }
    janela?.removeEventListener('pagehide', limpar);
    janela?.removeEventListener('popstate', verificarContexto);
    if (documento.__descobertaPublica === promessa) delete documento.__descobertaPublica;
  };
  const verificarContexto = () => { if (!contextoPublico(documento, localizacao)) limpar(); };
  const verificarExecucao = sinal => controlador.signal.aborted || sinal?.aborted ? falha('execucao_cancelada') : !contextoPublico(documento, localizacao) ? falha('contexto_nao_publico') : null;
  const ferramentas = [{
    name: nomes[0],
    description: 'Consulta o título, a descrição e os dados estruturados públicos da página aberta.',
    inputSchema: { type: 'object', properties: {}, additionalProperties: false },
    async execute(entrada, { signal } = {}) {
      const impedimento = verificarExecucao(signal);
      if (impedimento) return impedimento;
      if (!entrada || typeof entrada !== 'object' || Array.isArray(entrada) || Object.keys(entrada).length) return falha('entrada_invalida');
      const schemas = [], problemas = [];
      for (const script of [...documento.querySelectorAll('script[type="application/ld+json"]')].slice(0, 20)) {
        if (script.closest('[data-descoberta-privada], [hidden], [aria-hidden="true"], form')) continue;
        const json = script.textContent || '';
        if (json.length > 100_000) { problemas.push({ codigo: 'json_ld_grande' }); continue; }
        try {
          const valor = JSON.parse(json);
          if (valor && typeof valor === 'object') schemas.push(valor);
          else problemas.push({ codigo: 'json_ld_invalido' });
        } catch { problemas.push({ codigo: 'json_ld_invalido' }); }
      }
      const descricao = documento.head?.querySelector('meta[name="description"]')?.getAttribute('content') || '';
      return { url: enderecoPublico(localizacao.href, new URL(localizacao.href).origin), titulo: (documento.title || '').slice(0, 500), descricao: descricao.slice(0, 2000), schemas, problemas };
    },
    annotations: { readOnlyHint: true },
  }, {
    name: nomes[1],
    description: 'Localiza links públicos da navegação desta página por assunto, sem abrir destinos.',
    inputSchema: { type: 'object', properties: { consulta: { type: 'string', minLength: 1, maxLength: 120, description: 'Assunto procurado na navegação.' }, limite: { type: 'integer', minimum: 1, maximum: 30, default: 10 } }, required: ['consulta'], additionalProperties: false },
    async execute(entrada, { signal } = {}) {
      const impedimento = verificarExecucao(signal);
      if (impedimento) return impedimento;
      if (!entrada || typeof entrada !== 'object' || Array.isArray(entrada) || Object.keys(entrada).some(chave => !['consulta', 'limite'].includes(chave)) || typeof entrada.consulta !== 'string' || !entrada.consulta.trim() || entrada.consulta.length > 120) return falha('entrada_invalida');
      const limite = entrada.limite ?? 10;
      if (!Number.isInteger(limite) || limite < 1 || limite > 30) return falha('entrada_invalida');
      const origem = new URL(localizacao.href).origin;
      const consulta = entrada.consulta.trim().toLowerCase();
      const links = new Map();
      for (const ancora of documento.querySelectorAll('nav a[href], header a[href], [role="navigation"] a[href]')) {
        if (ancora.closest('[data-descoberta-privada], [hidden], [aria-hidden="true"], form')) continue;
        const url = enderecoPublico(ancora.href || new URL(ancora.getAttribute('href'), localizacao.href).href, origem);
        if (!url) continue;
        const titulo = textoPublico(ancora).replace(/\s+/g, ' ').trim().slice(0, 200);
        if (titulo && `${titulo} ${new URL(url).pathname}`.toLowerCase().includes(consulta) && !links.has(url)) links.set(url, { titulo, url });
      }
      const ordenados = [...links.values()].sort((a, b) => a.url < b.url ? -1 : a.url > b.url ? 1 : 0);
      return { links: ordenados.slice(0, limite) };
    },
    annotations: { readOnlyHint: true },
  }];
  promessa = (async () => {
    try {
      controlador.signal.addEventListener('abort', limpar, { once: true });
      for (const ferramenta of ferramentas) {
        if (controlador.signal.aborted) throw new Error('Registro cancelado.');
        await api.registerTool(ferramenta, { signal: controlador.signal });
        registrados.push(ferramenta.name);
        if (controlador.signal.aborted) {
          if (typeof api.unregisterTool === 'function') { try { api.unregisterTool(ferramenta.name); } catch { /* Compatibilidade com registro antigo já retirado. */ } }
          throw new Error('Registro cancelado.');
        }
      }
      janela?.addEventListener('pagehide', limpar, { once: true });
      janela?.addEventListener('popstate', verificarContexto);
      return { ativo: true, nomes: [...nomes], desinstalar: limpar };
    } catch {
      limpar();
      return inativo('registro_falhou');
    }
  })();
  documento.__descobertaPublica = promessa;
  const resultado = await promessa;
  if (!resultado.ativo && documento.__descobertaPublica === promessa) delete documento.__descobertaPublica;
  return resultado;
}
