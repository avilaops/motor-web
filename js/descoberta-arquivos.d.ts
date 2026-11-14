import type { PaginaPublica } from './descoberta.d.ts';
export interface PublicacaoDescoberta {
  gerador: string;
  versao: number;
  origem: string;
  paginas: Array<PaginaPublica & { rota: string }>;
  ignoradas: Array<{ rota?: string; arquivo?: string; motivo: string }>;
  problemas: Array<{ rota: string; codigo: string; mensagem: string }>;
  arquivos: Array<{ caminho: string; sha256: string }>;
}
export function publicarDescoberta(opcoes: {
  diretorio: string; origem: string; nome?: string; descricao?: string; excluirPrefixos?: string[];
}): Promise<PublicacaoDescoberta>;
