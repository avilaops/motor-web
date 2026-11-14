export interface PaginaPublica {
  url: string;
  titulo: string;
  descricao?: string;
  consultas?: string[];
  schemas?: object[];
}
export interface RecursoPublico {
  url: string;
  nome: string;
  descricao?: string;
  tipo: string;
  consultas?: string[];
}
export interface EntradaArd {
  identifier: string;
  displayName: string;
  type: string;
  url: string;
  description: string;
  representativeQueries: string[];
}
export interface CatalogoPublico {
  ard: { entries: EntradaArd[] };
  aiCatalogo: { specVersion: '1.0'; host: { displayName: string; identifier: string }; entries: EntradaArd[] };
  jsonArd: string;
  jsonAiCatalogo: string;
}
export function criarCatalogoPublico(opcoes: {
  origem: string; nome: string; descricao: string; paginas?: PaginaPublica[]; recursos?: RecursoPublico[];
}): CatalogoPublico;
export function analisarPaginaPublica(html: string, opcoes: { url: string }): {
  publica: boolean; motivo: string | null; url: string; titulo: string; descricao: string;
  schemas: object[]; problemas: Array<{ codigo: string; mensagem: string }>;
};
export function injetarDescoberta(html: string, opcoes: {
  url: string; catalogoUrl?: string; moduloUrl?: string; remover?: boolean;
}): string;
export function serializarJsonParaScript(valor: unknown): string;
