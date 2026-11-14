#!/usr/bin/env node
import { parseArgs } from 'node:util';
import { publicarDiretorio } from '../js/arquivos.mjs';

try {
  const { values } = parseArgs({ options: { diretorio: { type: 'string' }, origem: { type: 'string' } } });
  if (!values.diretorio || !values.origem) throw new Error('Uso: publicar-texto --diretorio <artefato> --origem <https://dominio>');
  const resultado = await publicarDiretorio(values);
  process.stdout.write(`${JSON.stringify({ paginas: resultado.paginas.length, ignoradas: resultado.ignoradas.length })}\n`);
} catch (erro) {
  process.stderr.write(`Falha ao gerar versões de texto: ${erro.message}\n`);
  process.exitCode = 1;
}
