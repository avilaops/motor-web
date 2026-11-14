// Sem API ou opt-in público, nenhuma importação adicional é iniciada.
if (typeof document !== 'undefined' && typeof location !== 'undefined') {
  const api = document.modelContext ?? (typeof navigator !== 'undefined' ? navigator.modelContext : undefined);
  if (typeof api?.registerTool === 'function' && document.head?.querySelector('meta[name="descoberta-publica"][content="true"][data-descoberta-publica="true"]')) {
    const iniciar = () => import('./descoberta-navegador.mjs').then(({ instalarFerramentasPublicas }) => instalarFerramentasPublicas({ documento: document, modelContext: api, localizacao: location, controlador: new AbortController() })).catch(() => {});
    void iniciar();
    document.defaultView?.addEventListener('pageshow', evento => { if (evento.persisted) void iniciar(); });
  }
}
