export interface MetadadosPublicados {
  generator: string;
  title: string;
  description?: string;
  canonical: string;
  language?: string;
  updated?: string;
}
export interface TextoPublicado {
  conteudo: string;
  metadados: MetadadosPublicados;
  indexavel: boolean;
  motivo: 'rota_privada' | 'noindex' | 'sem_conteudo' | null;
  links: string[];
}
export function converterHtml(html: string, opcoes: { url: string; formato?: 'md' | 'mdx' }): TextoPublicado;
export function caminhosDeTexto(url: string): { md: string; mdx: string };
