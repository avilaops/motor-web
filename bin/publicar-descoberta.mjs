#!/usr/bin/env node
import { parseArgs } from 'node:util';
import { publicarDescoberta } from '../js/descoberta-arquivos.mjs';

try {
  const { values } = parseArgs({ options: {
    diretorio: { type: 'string' }, origem: { type: 'string' }, nome: { type: 'string' }, descricao: { type: 'string' },
    'excluir-prefixo': { type: 'string', multiple: true },
  } });
  if (!values.diretorio || !values.origem) throw new Error('Uso: publicar-descoberta --diretorio <artefato> --origem <https://dominio> [--excluir-prefixo /rota/]');
  const resultado = await publicarDescoberta({ diretorio: values.diretorio, origem: values.origem, nome: values.nome, descricao: values.descricao, excluirPrefixos: values['excluir-prefixo'] || [] });
  process.stdout.write(`${JSON.stringify({ paginas: resultado.paginas.length, ignoradas: resultado.ignoradas.length, problemas: resultado.problemas.length })}\n`);
} catch (erro) {
  process.stderr.write(`Falha ao publicar descoberta: ${erro.message}\n`);
  process.exitCode = 1;
}
