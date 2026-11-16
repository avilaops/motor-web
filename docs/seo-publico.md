# Publicação SEO do artefato estático

O CLI `publicar-seo` prepara metadados, canonical, JSON-LD de página, sitemaps de páginas/imagens, índice de sitemaps e robots permissivo para páginas públicas.

```sh
node bin/publicar-seo.mjs --diretorio out --configuracao seo.config.json --repositorio .
node bin/publicar-texto.mjs --diretorio out --origem https://oficina.example --indexar-texto
node bin/publicar-descoberta.mjs --diretorio out --origem https://oficina.example
```

Configuração fictícia:

```json
{
  "origem": "https://oficina.example",
  "nome": "Oficina exemplo",
  "descricao": "Serviços e informações públicas da oficina.",
  "idioma": "pt-BR",
  "imagem": "/marca.png",
  "icone": "/favicon.svg",
  "removerTagsLegadas": true,
  "excluirRotas": ["/interno", "/obrigado"],
  "fontesGlobais": ["seo.config.json"],
  "fontesPorRota": {"/": ["index.html"], "/contato.html": ["contato.html"]}
}
```

O núcleo `js/seo.mjs` não acessa disco, Git, rede ou relógio. O adaptador resolve arquivos dentro do artefato e obtém a última data do histórico Git das fontes informadas. Sem uma fonte verificável, omite `lastmod`; nunca usa a data de execução do build. O checkout precisa ter o histórico necessário para que a data represente a última mudança do conteúdo.

URLs HTML legadas recebem canonical sem a extensão, conforme o comportamento do Pages. Rotas privadas e páginas de erro não entram nos sitemaps. `excluirRotas` acrescenta restrições específicas do consumidor. A imagem/ícone de marca local precisa existir no artefato.

`removerTagsLegadas` retira carregadores diretos conhecidos do Google de HTML estático. Não remove scripts de hidratação Next.js: nesses projetos, o carregador deve ser retirado na fonte do layout.

O comportamento padrão de `publicar-texto` continua com `noindex` nas cópias textuais. A opção explícita `--indexar-texto` usa `index,follow` e o cabeçalho HTTP `Link` apontando para o HTML canônico, permitindo descoberta sem anunciar cópias duplicadas no sitemap.

O publicador não garante indexação, citações por IA ou conversão. Configuração da borda, mensuração, consentimento, disponibilidade de imagens e processamento de buscadores exigem validação no ambiente publicado.
