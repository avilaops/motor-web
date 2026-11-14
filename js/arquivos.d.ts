export interface ManifestoPublicacao {
  gerador: string;
  versao: number;
  paginas: Array<{ rota: string; url: string; md: string; mdx: string; title: string; updated?: string }>;
  ignoradas: Array<{ rota?: string; arquivo?: string; motivo: string }>;
}
export function publicarDiretorio(opcoes: { diretorio: string; origem: string }): Promise<ManifestoPublicacao>;
