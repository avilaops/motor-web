export interface FerramentaPublica {
  name: string;
  description: string;
  inputSchema: object;
  execute(entrada: object, opcoes?: { signal?: AbortSignal }): Promise<object>;
  annotations: { readOnlyHint: true };
}
export interface ModelContextPublico {
  registerTool(ferramenta: FerramentaPublica, opcoes?: { signal?: AbortSignal }): void | Promise<void>;
  unregisterTool?(nome: string): void;
}
export interface InstalacaoPublica {
  ativo: boolean;
  motivo?: string;
  nomes: string[];
  desinstalar(): void;
}
export function instalarFerramentasPublicas(opcoes: {
  documento: Document;
  modelContext?: ModelContextPublico;
  navegador?: { modelContext?: ModelContextPublico };
  localizacao: { href: string };
  controlador: AbortController;
}): Promise<InstalacaoPublica>;
export function origemCanonica(valor: string): string;
export function enderecoPublico(valor: string, origem: string): string | undefined;
