#!/usr/bin/env node
import { parseArgs } from 'node:util';
import { readFile } from 'node:fs/promises';
import { publicarSeo } from '../js/seo-arquivos.mjs';

try {
  const { values } = parseArgs({ options: { diretorio: { type: 'string' }, configuracao: { type: 'string' }, repositorio: { type: 'string' } } });
  if (!values.diretorio || !values.configuracao) throw new Error('Uso: publicar-seo --diretorio <artefato> --configuracao <arquivo.json> [--repositorio <checkout>]');
  const configuracao = JSON.parse(await readFile(values.configuracao, 'utf8'));
  const resultado = await publicarSeo({ diretorio: values.diretorio, configuracao, repositorio: values.repositorio });
  process.stdout.write(JSON.stringify({ paginas: resultado.paginas.length, ignoradas: resultado.ignoradas.length }) + '\n');
} catch (erro) {
  process.stderr.write('Falha ao publicar SEO: ' + erro.message + '\n');
  process.exitCode = 1;
}
