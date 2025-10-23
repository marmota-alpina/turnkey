# Guia Completo do Protocolo Henry - Emulador de Equipamentos

## 1. Visão Geral do Sistema

### 1.1 Equipamentos Suportados
- **Primme Acesso** (versões 1.0.0.23 e 8.0.0.50)
- **Argos**
- **Primme SF (Super Fácil)**
- **Catracas com leitores RFID e Biométricos**

### 1.2 Protocolo de Comunicação
- Comunicação TCP/IP
- Formato de mensagem: `ID+REON+CODIGO+DADOS`
- Separadores de campo: `]`, `[`, `+`, `{`, `}`
- Codificação: ASCII
- Estrutura geral: `<SB><XXXX><II>+COMANDO+00+DADOS<CS><EB>`

## 2. Fluxo de Comunicação das Catracas

### 2.1 Fluxo Completo com Confirmação de Giro

#### Sequência de Comandos:

1. **Solicitação da Catraca**
   ```
   15+REON+000+0]00000000000011912322]10/05/2016 12:46:06]1]0]
   ```
   - `15`: ID do equipamento
   - `000+0`: Código do comando (solicitação de acesso)
   - `00000000000011912322`: Número do cartão/matrícula
   - `10/05/2016 12:46:06`: Data/hora do evento
   - `1`: Direção (1=entrada, 2=saída)
   - `0`: Indicador adicional

2. **Resposta do Software - Acesso Liberado**
   ```
   15+REON+00+6]5]Acesso liberado]
   ```
   - `00+6`: Código para liberar saída
   - `5`: Tempo de liberação em segundos
   - `Acesso liberado`: Mensagem para display

3. **Resposta da Catraca - Aguardando Giro**
   ```
   15+REON+000+80]]10/05/2016 12:46:06]0]0]
   ```
   - `000+80`: Código indicando aguardando giro
   - Status: Catraca liberada aguardando usuário girar

4. **Simulação/Detecção do Giro**
   - Sensor detecta movimento do braço da catraca
   - Usuário inicia o giro físico

5. **Resposta da Catraca - Giro Completado**
   ```
   15+REON+000+81]]11/05/2016 14:33:24]2]0]
   ```
   - `000+81`: Código de giro completado
   - `2`: Direção do giro realizado

### 2.2 Fluxo com Desistência de Giro

1. **Solicitação da Catraca** (idêntico ao fluxo anterior)
   ```
   15+REON+000+0]00000000000011912322]10/05/2016 12:46:06]1]0]
   ```

2. **Resposta do Software - Acesso Liberado**
   ```
   15+REON+00+6]5]Acesso liberado]
   ```

3. **Resposta da Catraca - Aguardando Giro**
   ```
   15+REON+000+80]]10/05/2016 12:46:06]0]0]
   ```

4. **Resposta da Catraca - Desistência de Giro**
   ```
   15+REON+000+82]]11/05/2016 15:26:03]0]0]
   ```
   - `000+82`: Código de desistência (tempo limite expirado)

5. **Software Libera Novamente (Manual)**
   ```
   01+REON+00+4]5]Acesso liberado]
   ```
   - `00+4`: Liberação manual

6. **Catraca Aguarda Novamente**
   ```
   01+REON+000+80]]10/05/2016 12:46:06]0]0]
   ```

7. **Giro Efetivado**
   ```
   08+REON+000+81]]11/05/2016 15:26:03]0]0]
   ```

## 3. Códigos de Comando e Resposta

### 3.1 Códigos de Liberação

| Código | Descrição | Uso |
|--------|-----------|-----|
| `00+1` | Libera ambos os lados | Acesso bidirecional |
| `00+5` | Libera entrada | Acesso de entrada |
| `00+6` | Libera saída | Acesso de saída |
| `00+4` | Liberação manual | Liberação forçada pelo software |
| `00+30` | Acesso negado | Bloqueio de acesso |

### 3.2 Códigos de Status da Catraca

| Código | Descrição | Significado |
|--------|-----------|-------------|
| `000+0` | Solicitação | Catraca solicita validação |
| `000+80` | Aguardando giro | Catraca liberada esperando movimento |
| `000+81` | Giro completado | Passagem realizada com sucesso |
| `000+82` | Desistência | Tempo limite sem giro |

### 3.3 Identificação do Tipo de Leitora

O último campo do comando de solicitação indica o tipo de leitora utilizada:

| Valor | Tipo de Leitora |
|-------|-----------------|
| `1` | Leitora de proximidade RFID |
| `5` | Leitora biométrica |

## 4. Formato de Dados dos Comandos

### 4.1 Estrutura de Validação Online

#### Solicitação do Equipamento:
```
ID+REON+000+0]MATRICULA]DATA_HORA]DIRECAO]INDICADOR]TIPO_LEITORA
```

#### Resposta do Software:
```
ID+REON+00+CODIGO_LIBERACAO]TEMPO]MENSAGEM]
```

Onde:
- `ID`: Identificador do equipamento (01-99)
- `MATRICULA`: Número do cartão ou matrícula (até 20 dígitos)
- `DATA_HORA`: Formato dd/mm/aaaa hh:mm:ss
- `DIRECAO`: 1=entrada, 2=saída, 0=indefinido
- `TEMPO`: Tempo de liberação em segundos
- `MENSAGEM`: Texto para exibir no display (máximo 40 caracteres)

### 4.2 Exemplo de Comunicação Completa

```
// Cartão RFID aproximado
Catraca → Software: 01+REON+00+0]12651543]22/08/2011 08:57:01]1]0]1

// Software libera acesso
Software → Catraca: 01+REON+00+1]5]Acesso liberado]

// Catraca confirma liberação
Catraca → Software: 01+REON+000+80]]22/08/2011 08:57:01]0]0]

// Usuário passa pela catraca
Catraca → Software: 01+REON+000+81]]22/08/2011 08:57:02]1]0]
```

## 5. Comandos de Gerenciamento

### 5.1 Comandos Principais

| Código | Nome | Descrição | Primme | Argos | Primme SF |
|--------|------|-----------|--------|-------|-----------|
| EC | Configurações | Envia configurações ao equipamento | ✓ | ✓ | ✓ |
| EE | Empregador | Envia empregador ao equipamento | ✓ | ✗ | ✗ |
| EU | Usuário | Envia lista de usuários | ✓ | ✗ | ✗ |
| EH | Data e hora | Envia data e hora ao equipamento | ✓ | ✓ | ✓ |
| ED | Digitais | Envia lista de digitais | ✓ | ✓ | ✓ |
| ER | Registros | Recebe os registros | ✓ | ✓ | ✓ |
| ECAR | Cartão | Envia lista de cartões | ✓ | ✓ | ✓ |
| EACI | Acionamento | Envia lista de acionamentos | ✓ | ✗ | ✗ |
| EPER | Períodos | Envia lista de períodos | ✓ | ✗ | ✗ |
| EHOR | Horários | Envia lista de horários | ✓ | ✗ | ✗ |
| EFER | Feriados | Envia lista de feriados | ✓ | ✗ | ✗ |
| EMSG | Mensagens | Envia mensagens padrão | ✓ | ✗ | ✗ |
| EGA | Grupo de Acesso | Envia grupos de acesso | ✓ | ✗ | ✗ |
| ECGA | Cartões de GA | Envia cartões de grupo de acesso | ✓ | ✗ | ✗ |
| EFUN | Funções | Envia funções | ✓ | ✗ | ✗ |

### 5.2 Comandos de Recepção

| Código | Nome | Descrição |
|--------|------|-----------|
| RC | Configurações | Recebe configurações do equipamento |
| RE | Empregador | Recebe empregador do equipamento |
| RQ | Quantidade/Status | Recebe quantidades e status |

## 6. Configurações do Equipamento

### 6.1 Parâmetros Principais

#### Gerais
- `NR_EQUIP`: Número do equipamento (0-4294967295)
- `VOLUME`: Volume dos avisos sonoros (2-9, padrão: 9)
- `MSG_DISPLAY`: Mensagem do display (até 40 caracteres)
- `GER_INTELIGENTE`: Gerenciamento inteligente (H/D)
- `SENHA_MENU`: Senha do menu (9 dígitos Primme, 6 dígitos Argos)
- `LOGIN`: Usuário para acesso web (até 20 caracteres)

#### Validação e Acesso
- `TIPO_VALIDA`: Tipo de validação
  - `F`: Offline
  - `O`: Online
  - `A`: Automático
  - `S`: Semi-automático
- `ARMAZENA_REGISTRO`: Registros gravados (T/N/G/L)
- `TIMEOUT_ON`: Tempo limite online (500-10000ms, padrão: 3000)
- `ESPERA_OFF`: Tempo de espera offline (2-600s, padrão: 60)
- `TEMPO_PASSBACK`: Anti-passback em minutos (0-999999)
- `DIRECAO_PASSBACK`: Direção anti-passback (H/D)
- `VERIF_VALIDADE`: Verifica validade dos cartões (H/D)
- `ACESSO_USUARIO`: Tipo de acesso dos usuários (B/V/L)

#### Leitoras
- `LEITOR_1`, `LEITOR_2`, `LEITOR_3`: Configuração das leitoras
- `LEITOR_VER_DIG`: Solicita biometria ao ler cartão (H/D)
- `MODO_CADASTRO`: Cadastro automático (A/N)

## 7. Protocolo de Envio de Dados

### 7.1 Estrutura de Envio de Cartões

```
<SB><XXXX><II>+ECAR+00+QTD+OPERACAO[INDICE[CARTAO[VALIDADE_INI[VALIDADE_FIM[CODIGO[TIPO[VERIFICA_DIG[SENHA[SENHA_PANICO[RELES[SEQUENCIA[POSICAO[QTD_HORARIOS[HORARIOS[QTD_ESCALAS[ESCALAS[SENHA_SEGURA<CS><EB>
```

Onde:
- `QTD`: Quantidade de cartões
- `OPERACAO`: I=Inclusão, E=Exclusão, A=Alteração, L=Limpar lista
- `INDICE`: Índice do usuário
- `CARTAO`: Número do cartão (3-20 caracteres)
- `VALIDADE_INI/FIM`: dd/mm/aaaa hh:mm:ss
- `TIPO`: Tipo do cartão
- `VERIFICA_DIG`: H/D para verificação de digital

### 7.2 Estrutura de Envio de Usuários

```
<SB><XXXX><II>+EU+00+QTD+OPERACAO[INDICE[NOME[RESERVADO[QTD_REF[CARTOES<CS><EB>
```

Exemplo:
```
+EU+00+1+I[1001[João Silva[0[2[12345}67890
```

## 8. Protocolo de Biometria

### 8.1 Adição de Digital

```
<SB><XXXX><II>+ED+00+D]MATRICULA}QTD_TEMPLATES}NUM_DEDO{TEMPLATE<CS><EB>
```

### 8.2 Exclusão de Digital

```
<SB><XXXX><II>+ED+00+E]MATRICULA<CS><EB>
```

### 8.3 Limpar Todas as Digitais

```
<SB><XXXX><II>+ED+00+C]<CS><EB>
```

## 9. Protocolo de Eventos e Registros

O protocolo Henry suporta múltiplos métodos para recuperar logs de acesso dos dispositivos. O código de comando é `RR` (para Primme SF/Argos) ou `ER` (para Primme Acesso).

### 9.1 Métodos de Coleta de Registros

O protocolo fornece **5 modos de filtro** para recuperar logs de acesso, cada um otimizado para diferentes casos de uso:

#### 9.1.1 Filtro por Endereço de Memória (M)

Recupera eventos de uma localização específica de endereço de memória.

**Estrutura do Comando**:
```
<SB><XXXX><II>+RR+00+M]QUANTIDADE]ENDERECO_INICIAL<CS><EB>
```

**Campos**:
- `QUANTIDADE`: Número de eventos a recuperar
- `ENDERECO_INICIAL`: Endereço de memória para iniciar (baseado em 0)

**Exemplo**:
```
01+RR+00+M]3]0
```
Recupera 3 eventos começando do endereço de memória 0

**Caso de Uso**: Acesso de memória de baixo nível, útil para depuração ou varreduras completas de memória.

#### 9.1.2 Filtro por NSR (N)

Recupera eventos por número de registro sequencial (NSR - Número Sequencial de Registro).

**Estrutura do Comando**:
```
<SB><XXXX><II>+RR+00+N]QUANTIDADE]NSR_INICIAL<CS><EB>
```

**Campos**:
- `QUANTIDADE`: Número de eventos a recuperar
- `NSR_INICIAL`: Número NSR inicial (ID sequencial)

**Exemplo**:
```
01+RR+00+N]5]1
```
Recupera 5 eventos começando do NSR 1

**Caso de Uso**: Coleta sequencial com ordem garantida, ideal para processamento em lote.

#### 9.1.3 Filtro por Intervalo de Data (D)

Recupera eventos dentro de um intervalo específico de data/hora.

**Estrutura do Comando**:
```
<SB><XXXX><II>+RR+00+D]QUANTIDADE]DATA_HORA_INICIAL]DATA_HORA_FINAL<CS><EB>
```

**Campos**:
- `QUANTIDADE`: Número de eventos a recuperar
- `DATA_HORA_INICIAL`: Data/hora inicial (dd/mm/aaaa HH:MM:SS)
- `DATA_HORA_FINAL`: Data/hora final (opcional para Primme SF, obrigatório para Primme Acesso)

**Exemplo (Primme SF)**:
```
01+RR+00+D]2]10/07/2012 08:00:01]
```
Recupera 2 eventos após 10 de julho de 2012 08:00:01

**Exemplo (Primme Acesso)**:
```
01+ER+00+D]10]01/01/2024 00:00:00]31/01/2024 23:59:59
```
Recupera 10 eventos entre 1-31 de janeiro de 2024

**Caso de Uso**: Análise histórica, relatórios de conformidade, investigação de incidentes específicos.

#### 9.1.4 Filtro por Índice (T)

Recupera eventos por sua posição de índice sequencial.

**Estrutura do Comando**:
```
<SB><XXXX><II>+RR+00+T]QUANTIDADE]INDICE_INICIAL<CS><EB>
```

**Campos**:
- `QUANTIDADE`: Número de eventos a recuperar
- `INDICE_INICIAL`: Índice inicial (baseado em 1)

**Exemplo**:
```
01+RR+00+T]5]1
```
Recupera 5 eventos começando do índice 1

**Caso de Uso**: Paginação, recuperação sequencial com posições de índice conhecidas.

#### 9.1.5 Filtro por Não Coletados (C)

Recupera apenas eventos que ainda não foram coletados pelo software de gerenciamento.

**Estrutura do Comando**:
```
<SB><XXXX><II>+RR+00+C]QUANTIDADE]INDICE_INICIAL<CS><EB>
```

**Campos**:
- `QUANTIDADE`: Número de eventos não coletados a recuperar
- `INDICE_INICIAL`: Índice inicial (baseado em 0)

**Exemplo**:
```
01+RR+00+C]5]0
```
Recupera 5 eventos não coletados começando do índice 0

**Caso de Uso**: Sincronização incremental, garantindo que nenhum evento seja perdido durante interrupções de rede.

**Nota**: O dispositivo marca internamente os eventos como "coletados" após recuperação bem-sucedida. Use este modo para sincronização confiável.

### 9.2 Confirmação de Coleta

Após recuperar eventos com sucesso, o cliente deve enviar uma confirmação para marcá-los como coletados.

**Estrutura do Comando**:
```
<SB><XXXX><II>+ER+00+QTD_COLETADOS+INDICES]<CS><EB>
```

**Campos**:
- `QTD_COLETADOS`: Número de eventos coletados com sucesso
- `INDICES`: Lista separada por vírgulas de índices de eventos coletados

**Exemplo**:
```
01+ER+00+5+1,2,3,4,5]
```
Confirma coleta de 5 eventos com índices 1 a 5

### 9.3 Formato de Registro de Evento

Cada registro de evento retornado pelo dispositivo contém:

**Campos Padrão**:
- NSR (Número de Registro Sequencial)
- Data/Hora (dd/mm/aaaa HH:MM:SS)
- Número do Cartão/Matrícula
- Tipo de Evento (acesso liberado, negado, giro completado, etc.)
- Direção (1=entrada, 2=saída, 0=indefinido)
- Tipo de Leitora (1=RFID, 5=Biométrica)
- Metadados adicionais (varia por modelo de dispositivo)

**Exemplo de Resposta**:
```
01+RR+00+3+1}12345}22/08/2011 08:57:01}Acesso Liberado}1}1+2}67890}22/08/2011 09:15:22}Acesso Negado}2}1+3}11111}22/08/2011 10:30:45}Acesso Liberado}1}5
```

### 9.4 Melhores Práticas para Coleta de Eventos

1. **Use Modo Não Coletados (C) para Sincronização em Tempo Real**: Consulte periodicamente por eventos não coletados para manter logs atualizados.

2. **Use Intervalo de Data (D) para Consultas Históricas**: Ao investigar períodos específicos ou gerar relatórios.

3. **Implemente Paginação**: Solicite lotes gerenciáveis (ex: 50-100 eventos) para evitar timeouts de rede.

4. **Sempre Envie Confirmação**: Marque eventos como coletados para evitar processamento duplicado.

5. **Trate Falhas de Rede**: Implemente lógica de retentativa com backoff exponencial para conexões não confiáveis.

6. **Monitore Contagem de Não Coletados**: Consulte regularmente `RQ+00+RNC` para detectar atraso na coleta.

## 10. Consulta de Quantidades e Status

O comando `RQ` (Request Query) permite que o software de gerenciamento consulte informações de status e capacidade do dispositivo. O protocolo suporta **12 tipos distintos de consulta** para monitorar saúde do dispositivo, uso de memória e status de periféricos.

### 10.1 Tipos de Consulta Disponíveis

A estrutura geral do comando é:
```
<SB><XXXX><II>+RQ+00+PARAMETRO<CS><EB>
```

#### 10.1.1 Contagem de Usuários (U)

Consulta o número total de usuários registrados na memória do dispositivo.

**Comando**:
```
01+RQ+00+U
```

**Exemplo de Resposta**:
```
01+RQ+00+U]35
```
Dispositivo possui 35 usuários registrados.

**Caso de Uso**: Monitorar tamanho do banco de dados de usuários, validar sincronização, verificar capacidade antes de importações em lote.

#### 10.1.2 Contagem de Cartões (C)

Consulta o número total de cartões registrados (credenciais RFID).

**Comando**:
```
01+RQ+00+C
```

**Exemplo de Resposta**:
```
01+RQ+00+C]142
```
Dispositivo possui 142 cartões registrados.

**Faixa Típica**: 0-999999999 (varia por modelo de dispositivo)

#### 10.1.3 Contagem de Biometria (D)

Consulta o número total de templates de impressão digital registrados.

**Comando**:
```
01+RQ+00+D
```

**Exemplo de Resposta**:
```
01+RQ+00+D]78
```
Dispositivo possui 78 templates de impressão digital armazenados.

**Faixa Típica**: 0-10000 (varia por modelo de dispositivo e capacidade de armazenamento)

#### 10.1.4 Capacidade Total de Biometria (TD)

Consulta o número máximo de templates de impressão digital que o dispositivo pode armazenar.

**Comando**:
```
01+RQ+00+TD
```

**Exemplo de Resposta**:
```
01+RQ+00+TD]3000
```
Dispositivo suporta até 3000 templates de impressão digital.

**Caso de Uso**: Verificações de capacidade pré-cadastro, planejamento de implementação biométrica.

#### 10.1.5 Contagem de Registros (R)

Consulta o número total de eventos de log de acesso armazenados na memória do dispositivo.

**Comando**:
```
01+RQ+00+R
```

**Exemplo de Resposta**:
```
01+RQ+00+R]5427
```
Dispositivo possui 5427 eventos de log de acesso.

**Faixa Típica**: 0-999999999

**Caso de Uso**: Monitorar uso de armazenamento de logs, agendar coleta de logs antes que a memória encha.

#### 10.1.6 Contagem de Registros Não Coletados (RNC)

Consulta o número de logs de acesso ainda não coletados pelo software de gerenciamento.

**Comando**:
```
01+RQ+00+RNC
```

**Exemplo de Resposta**:
```
01+RQ+00+RNC]23
```
Dispositivo possui 23 eventos não coletados pendentes de sincronização.

**Caso de Uso**: Crítico para sincronização incremental - indica quantos eventos estão aguardando. Se este número cresce continuamente, a coleta está atrasada.

#### 10.1.7 Status de Bloqueio do Dispositivo (TP)

Consulta se o dispositivo está administrativamente bloqueado (impedido de acesso).

**Comando**:
```
01+RQ+00+TP
```

**Valores de Resposta**:
- `A`: Dispositivo está bloqueado (Bloqueado)
- `D`: Dispositivo está desbloqueado (Operação normal)

**Exemplo de Resposta**:
```
01+RQ+00+TP]D
```
Dispositivo está desbloqueado e operacional.

**Caso de Uso**: Monitoramento de segurança, verificação de bloqueio de emergência.

#### 10.1.8 Erro de Comunicação MRP (MRPE)

Consulta se há erro de comunicação com o MRP (módulo de impressora).

**Comando**:
```
01+RQ+00+MRPE
```

**Valores de Resposta**:
- `0`: Sem erro, comunicação com impressora OK
- `1`: Erro de comunicação detectado

**Exemplo de Resposta**:
```
01+RQ+00+MRPE]0
```
Módulo de impressora está comunicando normalmente.

**Caso de Uso**: Diagnóstico de periféricos, solução de problemas de impressora.

**Nota**: Aplicável apenas a dispositivos com impressoras térmicas integradas (ex: relógios de ponto).

#### 10.1.9 Status do Empregador (SEMP)

Consulta se a informação do empregador está devidamente configurada no dispositivo.

**Comando**:
```
01+RQ+00+SEMP
```

**Valores de Resposta**:
- `0`: Empregador está registrado
- `1`: Empregador NÃO está registrado (configuração incompleta)

**Exemplo de Resposta**:
```
01+RQ+00+SEMP]0
```
Informação do empregador está configurada.

**Caso de Uso**: Validação de configuração inicial, verificações de conformidade (legislação trabalhista brasileira requer registro do empregador).

#### 10.1.10 Sensor de Papel Baixo (PP)

Consulta se o sensor de papel baixo está ativo (rolo de papel acabando).

**Comando**:
```
01+RQ+00+PP
```

**Valores de Resposta**:
- `0`: Nível de papel adequado
- `1`: Aviso de papel baixo ativo

**Exemplo de Resposta**:
```
01+RQ+00+PP]1
```
Papel está acabando, reabastecer em breve.

**Caso de Uso**: Alertas proativos de manutenção para sistemas de relógio de ponto.

#### 10.1.11 Status Sem Papel (SP)

Consulta se o dispositivo está completamente sem papel.

**Comando**:
```
01+RQ+00+SP
```

**Valores de Resposta**:
- `0`: Papel disponível
- `1`: Sem papel (vazio)

**Exemplo de Resposta**:
```
01+RQ+00+SP]0
```
Rolo de papel está presente.

**Caso de Uso**: Alertas críticos para funcionalidade de relógio de ponto, prevenir perda de registros.

#### 10.1.12 Capacidade de Papel (QP)

Consulta informações detalhadas de capacidade do rolo de papel.

**Comando**:
```
01+RQ+00+QP
```

**Formato de Resposta**:
```
01+RQ+00+QP]CAPACIDADE_TICKET]TAMANHO_ATUAL]TAMANHO_TOTAL
```

**Exemplo de Resposta**:
```
01+RQ+00+QP]500]350]500
```
- Capacidade de ticket: 500 impressões por rolo
- Tamanho atual: 350 impressões restantes
- Tamanho total: 500 impressões (rolo completo)

**Caso de Uso**: Monitoramento preciso de uso de papel, agendamento de manutenção preditiva.

### 10.2 Registros Offline Não Coletados (RNCO)

Consulta o número de logs de acesso em modo offline ainda não coletados.

**Comando**:
```
01+RQ+00+RNCO
```

**Exemplo de Resposta**:
```
01+RQ+00+RNCO]12
```
Dispositivo possui 12 eventos offline não coletados.

**Caso de Uso**: Rastrear eventos registrados durante interrupções de rede ou períodos de validação offline.

**Nota**: Este parâmetro é separado de `RNC` e rastreia especificamente eventos validados localmente quando o servidor estava inacessível.

### 10.3 Padrões de Consulta e Melhores Práticas

#### Sequência de Pesquisa de Monitoramento de Saúde

Para verificações abrangentes de saúde do dispositivo, consulte nesta ordem:

```
1. RQ+00+TP     (Dispositivo bloqueado?)
2. RQ+00+SEMP   (Empregador configurado?)
3. RQ+00+RNC    (Eventos pendentes para coletar?)
4. RQ+00+R      (Total de eventos armazenados)
5. RQ+00+U      (Contagem de usuários)
6. RQ+00+C      (Contagem de cartões)
7. RQ+00+D      (Contagem de biometria)
```

#### Consultas de Planejamento de Capacidade

Antes de operações em lote:

```
1. RQ+00+TD     (Capacidade máxima de biometria)
2. RQ+00+D      (Contagem atual de biometria)
   → Espaços disponíveis = TD - D

3. RQ+00+U      (Contagem de usuários)
4. RQ+00+C      (Contagem de cartões)
   → Validar proporções, planejar importações
```

#### Consultas de Manutenção de Impressora

Para dispositivos de relógio de ponto:

```
1. RQ+00+SP     (Sem papel?)
2. RQ+00+PP     (Aviso de papel baixo?)
3. RQ+00+QP     (Capacidade exata)
4. RQ+00+MRPE   (Comunicação com impressora OK?)
```

### 10.4 Tratamento de Respostas

Todas as respostas `RQ` seguem o formato:
```
<SB><XXXX><II>+RQ+00+PARAMETRO]VALOR<CS><EB>
```

**Respostas de Erro**:
- Se o parâmetro não é suportado: Dispositivo pode retornar resposta vazia ou código de erro
- Se o dispositivo está offline: Sem resposta (timeout)
- Se o parâmetro é válido mas dados não disponíveis: Pode retornar `]0` ou valor vazio

**Recomendações de Timeout**:
- Consultas padrão: 3000ms
- Consultas de papel/impressora: 5000ms (leituras de sensores de hardware podem ser mais lentas)
- Consultas de capacidade: 2000ms (buscas rápidas em memória)

## 11. Conjunto de Comandos Estendidos

Esta seção documenta comandos avançados descobertos através da análise do emulador cliente Java oficial do fabricante. Estes comandos fornecem recursos sofisticados de controle de acesso incluindo permissões baseadas em tempo, grupos de acesso, automação de relés e customização de display.

**Nota de Compatibilidade**: A maioria dos comandos estendidos é específica do Primme Acesso e não é suportada nos modelos Argos ou Primme SF. Sempre verifique a compatibilidade do dispositivo antes da implementação.

### 11.1 Grupos de Acesso (EGA/RGA)

Grupos de acesso permitem agrupamento lógico de usuários e cartões para gerenciamento centralizado de permissões. Ao invés de configurar cada cartão individualmente, cartões são atribuídos a grupos com regras de acesso, períodos de tempo e horários compartilhados.

#### 11.1.1 Enviar Grupo de Acesso (EGA)

Criar, atualizar ou excluir definições de grupo de acesso.

**Estrutura do Comando**:
```
<SB><XXXX><II>+EGA+00+QTD+MODO[ID_GRUPO[NOME_GRUPO[VALIDO_DE[VALIDO_ATE[CAMPO5[CAMPO6[CAMPO7[[[CAMPO9[[CAMPO11[[CAMPO13[[<CS><EB>
```

**Campos**:
- `QTD`: Número de grupos nesta mensagem (tipicamente 1)
- `MODO`: Modo de operação
  - `I`: Inserir novo grupo
  - `A`: Atualizar grupo existente
  - `E`: Excluir grupo
  - `L`: Limpar todos os grupos
- `ID_GRUPO`: Identificador único do grupo (6 dígitos, preenchido com zeros, ex: `000023`)
- `NOME_GRUPO`: Nome descritivo do grupo (máximo 40 caracteres)
- `VALIDO_DE`: Início da validade (dd/mm/aaaa HH:MM:SS)
- `VALIDO_ATE`: Fim da validade (dd/mm/aaaa HH:MM:SS)
- `CAMPO5-13`: Campos reservados/configuração específica do dispositivo

**Exemplo - Inserir**:
```
01+EGA+00+1+I[000023[Grupo Equipe Suporte[01/01/2010 00:00:01[30/12/2012 23:59:59[2[1[1[[[0[[0[[0[[
```
Cria grupo de acesso "Grupo Equipe Suporte" (ID 000023) válido de 2010 a 2012.

**Exemplo - Excluir**:
```
01+EGA+00+1+E[000023
```
Exclui grupo de acesso com ID 000023.

**Exemplo - Limpar Todos**:
```
01+EGA+00+0+L
```
Remove todos os grupos de acesso da memória do dispositivo.

**Caso de Uso**:
- Organizar usuários por departamento (ex: "Engenharia", "RH", "Segurança")
- Definir grupos de contratados com datas de expiração
- Implementar acesso de visitantes com restrições de tempo

#### 11.1.2 Receber Grupo de Acesso (RGA)

Consultar definições de grupo de acesso armazenadas no dispositivo.

**Estrutura do Comando**:
```
<SB><XXXX><II>+RGA+00+QTD]INDICE_INICIAL<CS><EB>
```

**Campos**:
- `QTD`: Número de grupos a recuperar
- `INDICE_INICIAL`: Índice inicial (baseado em 0)

**Exemplo**:
```
01+RGA+00+2]0
```
Recupera 2 grupos de acesso começando do índice 0.

**Formato de Resposta**:
```
01+RGA+00+2+000023[Grupo Equipe Suporte[01/01/2010 00:00:01[30/12/2012 23:59:59[...]+000024[Visitantes[...]
```

**Caso de Uso**: Auditar configurações de grupo existentes, verificar sincronização.

### 11.2 Associações Cartão-Grupo (ECGA/RCGA)

Vincular cartões individuais a grupos de acesso. Cada cartão pode ser associado com um ou mais grupos (dependendo do dispositivo).

#### 11.2.1 Enviar Associação Cartão-Grupo (ECGA)

Associar cartões com grupos de acesso.

**Estrutura do Comando**:
```
<SB><XXXX><II>+ECGA+00+QTD+MODO[ID_GRUPO[INDICE_CARTAO<CS><EB>
```

**Campos**:
- `QTD`: Número de associações nesta mensagem
- `MODO`:
  - `I`: Inserir associação
  - `E`: Excluir associação
  - `L`: Limpar todas as associações
- `ID_GRUPO`: ID do grupo de acesso (6 dígitos)
- `INDICE_CARTAO`: Índice do cartão na memória do dispositivo (baseado em 1)

**Exemplo - Associar Cartão**:
```
01+ECGA+00+1+I[000023[1
```
Associa cartão no índice 1 com grupo de acesso 000023.

**Exemplo - Múltiplas Associações**:
```
01+ECGA+00+3+I[000023[1+I[000023[2+I[000024[3
```
- Cartões 1 e 2 → Grupo 000023
- Cartão 3 → Grupo 000024

**Exemplo - Excluir Associação**:
```
01+ECGA+00+1+E[000023[1
```
Remove cartão 1 do grupo 000023.

**Caso de Uso**: Atualizações de permissão em lote, participação temporária em grupo, acesso baseado em função.

#### 11.2.2 Receber Associação Cartão-Grupo (RCGA)

Consultar quais cartões pertencem a quais grupos.

**Estrutura do Comando**:
```
<SB><XXXX><II>+RCGA+00+QTD]INDICE_INICIAL<CS><EB>
```

**Exemplo**:
```
01+RCGA+00+5]0
```
Recupera 5 associações cartão-grupo começando do índice 0.

**Caso de Uso**: Validar membros de grupo, auditar permissões de acesso.

### 11.3 Acionamentos de Relé (EACI/RACI)

Agendar ativações automáticas de relé (ex: desbloqueio de porta, ativação de alarme) baseadas em padrões de tempo e dia da semana. Útil para horários de abertura/fechamento automatizados.

#### 11.3.1 Enviar Acionamento de Relé (EACI)

Configurar ativações de relé agendadas.

**Estrutura do Comando**:
```
<SB><XXXX><II>+EACI+00+QTD+MODO[ID_ACIONAMENTO[NOME[HORA[NUM_RELE[DURACAO[DIAS_SEMANA<CS><EB>
```

**Campos**:
- `QTD`: Número de acionamentos nesta mensagem
- `MODO`: `I`=Inserir, `A`=Atualizar, `E`=Excluir, `L`=Limpar todos
- `ID_ACIONAMENTO`: Identificador único do acionamento (numérico)
- `NOME`: Nome descritivo (ex: "Sirene Almoço", "Porta Automática")
- `HORA`: Hora de ativação (HH:MM:SS)
- `NUM_RELE`: Número do relé a ativar (1-3, dependendo do dispositivo)
- `DURACAO`: Duração da ativação em segundos
- `DIAS_SEMANA`: Dias para ativar (bitmask: `2`=Seg, `3`=Ter, `4`=Qua, `5`=Qui, `6`=Sex, `7`=Sáb, `1`=Dom)

**Exemplo - Alarme de Almoço**:
```
01+EACI+00+1+I[13[Sirene Almoço[12:00:00[1[5[23456
```
Ativa relé 1 por 5 segundos às 12:00:00, segunda a sexta (23456).

**Exemplo - Abertura de Porta no Fim de Semana**:
```
01+EACI+00+1+I[20[Abertura Fim de Semana[08:00:00[2[10[17
```
Ativa relé 2 por 10 segundos às 08:00:00 no sábado e domingo (17).

**Exemplo - Excluir Acionamento**:
```
01+EACI+00+1+E[13
```
Exclui acionamento com ID 13.

**Caso de Uso**:
- Desbloqueio automático de porta durante horário comercial
- Ativação/desativação agendada de alarme
- Sinais de intervalo (sirenes, sinos)
- Integração com controle de HVAC/iluminação

#### 11.3.2 Receber Acionamento de Relé (RACI)

Consultar agendamentos de acionamento de relé configurados.

**Estrutura do Comando**:
```
<SB><XXXX><II>+RACI+00+QTD]INDICE_INICIAL<CS><EB>
```

**Exemplo**:
```
01+RACI+00+2]0
```
Recupera 2 acionamentos de relé começando do índice 0.

### 11.4 Períodos de Tempo (EPER/RPER)

Definir janelas de tempo (hora início, hora fim, dias ativos) usadas por grupos de acesso e horários. Períodos de tempo são componentes reutilizáveis referenciados por horários.

#### 11.4.1 Enviar Período de Tempo (EPER)

Criar definições de período de tempo.

**Estrutura do Comando**:
```
<SB><XXXX><II>+EPER+00+QTD+MODO[ID_PERIODO[HORA_INICIO[HORA_FIM[DIAS_SEMANA<CS><EB>
```

**Campos**:
- `QTD`: Número de períodos nesta mensagem
- `MODO`: `I`=Inserir, `A`=Atualizar, `E`=Excluir, `L`=Limpar todos
- `ID_PERIODO`: Identificador único do período (numérico)
- `HORA_INICIO`: Início do período (HH:MM:SS)
- `HORA_FIM`: Fim do período (HH:MM:SS)
- `DIAS_SEMANA`: Dias ativos (ex: `234567` = Seg-Dom, `23456` = Seg-Sex)

**Exemplo - Horário Comercial**:
```
01+EPER+00+1+I[1[08:00:00[18:00:00[23456
```
Período 1: 08:00-18:00, segunda a sexta.

**Exemplo - Turno Noturno**:
```
01+EPER+00+1+I[13[22:00:00[06:00:00[234567
```
Período 13: 22:00-06:00 (durante a noite), toda a semana.

**Nota**: Para períodos noturnos (hora fim < hora início), o período cruza a meia-noite.

**Exemplo - Acesso de Fim de Semana**:
```
01+EPER+00+1+I[5[09:00:00[17:00:00[17
```
Período 5: 09:00-17:00, apenas sábado e domingo.

**Caso de Uso**: Definir janelas de tempo reutilizáveis para controle de acesso, padrões de turno, janelas de manutenção.

#### 11.4.2 Receber Período de Tempo (RPER)

Consultar definições de período de tempo.

**Estrutura do Comando**:
```
<SB><XXXX><II>+RPER+00+QTD]INDICE_INICIAL<CS><EB>
```

**Exemplo**:
```
01+RPER+00+3]0
```
Recupera 3 períodos de tempo começando do índice 0.

### 11.5 Horários (EHOR/RHOR)

Horários nomeados que referenciam períodos de tempo. Horários fornecem um nome legível para combinações complexas de períodos de tempo.

#### 11.5.1 Enviar Horário (EHOR)

Criar horários nomeados referenciando períodos de tempo.

**Estrutura do Comando**:
```
<SB><XXXX><II>+EHOR+00+QTD+MODO[ID_HORARIO[NOME[CAMPO3[ID_PERIODO<CS><EB>
```

**Campos**:
- `QTD`: Número de horários nesta mensagem
- `MODO`: `I`=Inserir, `A`=Atualizar, `E`=Excluir, `L`=Limpar todos
- `ID_HORARIO`: Identificador único do horário (numérico)
- `NOME`: Nome do horário (máximo 40 caracteres)
- `CAMPO3`: Campo reservado (tipicamente `1`)
- `ID_PERIODO`: Referência ao ID do período de tempo (de EPER)

**Exemplo**:
```
01+EHOR+00+1+I[13[Horário da Tarde[1[13
```
Cria horário "Horário da Tarde" (ID 13) usando período 13.

**Exemplo - Múltiplos Horários**:
```
01+EHOR+00+2+I[1[Turno Diurno[1[1+I[2[Turno Noturno[1[13
```
- Horário 1 "Turno Diurno" usa período 1
- Horário 2 "Turno Noturno" usa período 13

**Caso de Uso**: Organizar acesso baseado em tempo (horários de turno, horários de visitantes, janelas de acesso de contratados).

#### 11.5.2 Receber Horário (RHOR)

Consultar definições de horário.

**Estrutura do Comando**:
```
<SB><XXXX><II>+RHOR+00+QTD]INDICE_INICIAL<CS><EB>
```

**Exemplo**:
```
01+RHOR+00+2]0
```
Recupera 2 horários começando do índice 0.

### 11.6 Feriados (EFER/RFER)

Registrar datas de feriados onde regras de acesso normais podem ser substituídas ou desabilitadas. Feriados são independentes de ano (ex: "01/01" aplica-se a todo 1º de janeiro).

#### 11.6.1 Enviar Feriado (EFER)

Registrar datas de feriados.

**Estrutura do Comando**:
```
<SB><XXXX><II>+EFER+00+QTD+MODO[DATA<CS><EB>
```

**Campos**:
- `QTD`: Número de feriados nesta mensagem
- `MODO`: `I`=Inserir, `E`=Excluir, `L`=Limpar todos
- `DATA`: Data do feriado no formato `dd/mm` (independente de ano)

**Exemplo - Ano Novo**:
```
01+EFER+00+1+I[01/01
```
Registra 1º de janeiro como feriado.

**Exemplo - Múltiplos Feriados**:
```
01+EFER+00+3+I[01/01+I[25/12+I[07/09
```
Registra 1º de janeiro, 25 de dezembro e 7 de setembro (Independência do Brasil).

**Exemplo - Excluir Feriado**:
```
01+EFER+00+1+E[25/12
```
Remove 25 de dezembro da lista de feriados.

**Caso de Uso**:
- Desabilitar desbloqueio automático de porta em feriados
- Substituir regras de grupo de acesso para feriados públicos
- Aplicar horários especiais em dias não úteis

#### 11.6.2 Receber Feriado (RFER)

Consultar feriados registrados, opcionalmente filtrados por mês.

**Estrutura do Comando**:
```
<SB><XXXX><II>+RFER+00+QTD+MODO/MES<CS><EB>
```

**Campos**:
- `QTD`: Número de feriados a recuperar
- `MODO`: Modo de filtro (tipicamente `0` para todos)
- `MES`: Número do mês (1-12) ou `0` para todos os meses

**Exemplo - Todos os Feriados em Janeiro**:
```
01+RFER+00+1+0/1
```
Recupera todos os feriados no mês 1 (janeiro).

**Exemplo - Todos os Feriados**:
```
01+RFER+00+10+0/0
```
Recupera até 10 feriados em todos os meses.

### 11.7 Mensagens de Display (EMSG/RMSG)

Personalizar as mensagens exibidas no LCD do dispositivo durante eventos de entrada e saída. Mensagens podem incluir texto estático e campos dinâmicos (nome do usuário, hora, etc.).

#### 11.7.1 Enviar Mensagens (EMSG)

Configurar mensagens padrão de entrada/saída.

**Estrutura do Comando**:
```
<SB><XXXX><II>+EMSG+00+MODO[MSG_ENTRADA[CAMPO_ENTRADA[[MSG_SAIDA[[CAMPO_SAIDA[<CS><EB>
```

**Campos**:
- `MODO`: Modo de mensagem
  - `2`: Mensagens personalizadas
  - Outros valores: Padrões específicos do dispositivo
- `MSG_ENTRADA`: Mensagem mostrada na entrada (máximo 40 caracteres)
- `CAMPO_ENTRADA`: Código de campo dinâmico para entrada
  - `5`: Exibir nome do usuário
  - `0`: Sem campo dinâmico
  - Outros códigos específicos do dispositivo (hora, ID do funcionário, etc.)
- `MSG_SAIDA`: Mensagem mostrada na saída (máximo 40 caracteres)
- `CAMPO_SAIDA`: Código de campo dinâmico para saída

**Exemplo - Mensagem de Boas-Vindas com Nome**:
```
01+EMSG+00+2[Bem Vindo[5[[Ate Logo[[5[
```
- Entrada: "Bem Vindo" + nome do usuário (campo 5)
- Saída: "Ate Logo" + nome do usuário (campo 5)

**Exemplo - Mensagens Estáticas**:
```
01+EMSG+00+2[ACESSO AUTORIZADO[0[[BOA VIAGEM[[0[
```
- Entrada: "ACESSO AUTORIZADO" (sem campo dinâmico)
- Saída: "BOA VIAGEM" (sem campo dinâmico)

**Exemplo - Mensagens Baseadas em Hora**:
```
01+EMSG+00+2[Bom Dia[3[[Saudacao[[5[
```
- Entrada: "Bom Dia" + campo 3 (possivelmente hora)
- Saída: "Saudacao" + nome do usuário

**Referência de Código de Campo** (dependente do dispositivo):
- `0`: Sem campo dinâmico (apenas texto estático)
- `3`: Hora ou data atual
- `5`: Nome do usuário
- `7`: ID do funcionário
- Outros códigos: Consulte manual do dispositivo

**Caso de Uso**: Experiência do usuário personalizada, mensagens multilíngues, marca.

#### 11.7.2 Receber Mensagens (RMSG)

Consultar configuração atual de mensagens de entrada/saída.

**Estrutura do Comando**:
```
<SB><XXXX><II>+RMSG+00<CS><EB>
```

**Exemplo**:
```
01+RMSG+00
```
Recupera mensagens de entrada e saída configuradas.

**Exemplo de Resposta**:
```
01+RMSG+00+2[Bem Vindo[5[[Ate Logo[[5[
```

**Caso de Uso**: Verificar configuração de mensagens, auditar configurações de display.

### 11.8 Processamento de Arquivo em Lote

O emulador cliente oficial suporta execução de comandos em lote a partir de arquivos de texto, permitindo testes automatizados, configuração em massa e gerenciamento de dispositivos com scripts.

#### 11.8.1 Formato de Arquivo

Arquivo de texto plano (extensão `.txt`) com um comando por linha. Comandos são executados sequencialmente.

**Arquivo de Exemplo** (`config.txt`):
```
EC+00+IP[192.168.1.100]
EC+00+PORTA[3000]
EC+00+TIPO_VALIDA[O]
EH+00+21/10/2025 10:30:00]00/00/00]00/00/00
RQ+00+U
RQ+00+C
```

Comandos:
1. Definir IP do dispositivo para 192.168.1.100
2. Definir porta para 3000
3. Definir modo de validação para Online
4. Definir data/hora
5. Consultar contagem de usuários
6. Consultar contagem de cartões

#### 11.8.2 Comandos Especiais

**Pause** - Aguardar pressionar tecla do usuário:
```
*pause*
```
Execução para até o usuário pressionar qualquer tecla. Útil para revisar saída antes de continuar.

**Sleep** - Aguardar milissegundos especificados:
```
*sleep*5000
```
Pausa a execução por 5000ms (5 segundos). Útil para aguardar após mudanças de configuração.

**Exemplo com Comandos Especiais**:
```
RH+00
RE+00
*pause*
EC+00+VOLUME[9]
*sleep*3000
RQ+00+U
```

Fluxo:
1. Consultar data/hora
2. Consultar empregador
3. PAUSE - aguardar usuário revisar
4. Definir volume para 9
5. SLEEP - aguardar 3 segundos para mudança de volume
6. Consultar contagem de usuários

#### 11.8.3 Modos de Execução em Lote

**Execução Única**:
- Carregar arquivo e executar uma vez
- Para no fim do arquivo ou em erro

**Modo de Loop**:
- Executar arquivo repetidamente
- Contagem de loop configurável (ex: 10 iterações) ou infinita
- Pressionar ESC para parar loops infinitos
- Útil para teste de stress, monitoramento contínuo

**Casos de Uso**:
- Provisionamento inicial de dispositivo (config em massa)
- Teste de regressão (executar suite de testes)
- Monitoramento contínuo de saúde (loop de comandos de consulta)
- Migração automática de dados (importação em massa de usuários/cartões)

**Exemplo - Script de Configuração de Dispositivo**:
```
# Configuração inicial
EC+00+NR_EQUIP[1]
EC+00+IP[192.168.1.100]
EC+00+TIPO_VALIDA[O]
EC+00+TIMEOUT_ON[3000]
*pause*

# Definir empregador
EE+00+2]00000000001]]Minha Empresa]Curitiba
*sleep*2000

# Verificar configuração
RC+00+IP
RC+00+TIPO_VALIDA
RQ+00+SEMP
*pause*

# Adicionar usuários iniciais
EU+00+1+I[111111111111[Admin[0[1[000001
*sleep*1000
ECAR+00+1+I[1[1[01/01/2025 00:00:01[31/12/2025 23:59:59[1[1[0[999999[000001[[BM[0[0[0[0[[0
```

## 12. Resumo de Cobertura de Comandos

Este resumo fornece uma visão abrangente de todos os comandos do protocolo Henry descobertos do emulador cliente oficial, organizados por status de implementação.

### 12.1 Comandos Totalmente Documentados (Envio + Recepção)

Estes comandos têm suporte bidirecional completo com especificações detalhadas de campos:

| Categoria | Comando Envio | Comando Recepção | Primme Acesso | Argos | Primme SF |
|-----------|---------------|------------------|---------------|-------|-----------|
| Configuração | EC | RC | ✓ | ✓ | ✓ |
| Empregador | EE | RE | ✓ | ✗ | ✗ |
| Usuários | EU | RU | ✓ | ✗ | ✗ |
| Data/Hora | EH | RH | ✓ | ✓ | ✓ |
| Cartões | ECAR | RCAR | ✓ | ✓ | ✓ |
| Grupos de Acesso | EGA | RGA | ✓ | ✗ | ✗ |
| Associações Cartão-Grupo | ECGA | RCGA | ✓ | ✗ | ✗ |
| Acionamentos de Relé | EACI | RACI | ✓ | ✗ | ✗ |
| Períodos de Tempo | EPER | RPER | ✓ | ✗ | ✗ |
| Horários | EHOR | RHOR | ✓ | ✗ | ✗ |
| Feriados | EFER | RFER | ✓ | ✗ | ✗ |
| Mensagens | EMSG | RMSG | ✓ | ✗ | ✗ |
| Eventos Online | REON | REON | ✓ | ✓ | ✓ |

### 12.2 Comandos de Consulta (Apenas Recepção)

Comandos de status e recuperação de dados:

| Comando | Propósito | Tipos de Consulta | Compatibilidade |
|---------|-----------|-------------------|-----------------|
| RQ | Quantidades e Status | 12 tipos (U, C, D, TD, R, RNC, RNCO, TP, MRPE, SEMP, PP, SP, QP) | Todos dispositivos |
| RR/ER | Logs de Acesso | 5 modos de filtro (M, N, D, T, C) | Todos dispositivos |

**Tipos de Consulta RQ**:
1. U - Contagem de usuários
2. C - Contagem de cartões
3. D - Contagem de biometria
4. TD - Capacidade total de biometria
5. R - Contagem de registros
6. RNC - Contagem de registros não coletados
7. RNCO - Registros offline não coletados
8. TP - Status de bloqueio do dispositivo
9. MRPE - Erro de comunicação MRP
10. SEMP - Status do empregador
11. PP - Sensor de papel baixo
12. SP - Status sem papel
13. QP - Capacidade de papel (detalhado)

**Modos de Filtro RR/ER**:
1. M - Filtro por endereço de memória
2. N - Filtro por NSR (número sequencial)
3. D - Filtro por intervalo de data
4. T - Filtro por índice
5. C - Filtro por status não coletado

### 12.3 Apenas Envio / Parcialmente Documentado

| Comando | Propósito | Nível de Documentação | Nota |
|---------|-----------|----------------------|------|
| ED | Enviar Biometria | Parcial | Formato de template proprietário, específico do dispositivo |
| RD | Receber Lista de Biometria | Parcial | Lista usuários com impressões digitais, dados de template não expostos |
| EFUN | Enviar Funções | Incompleto | Recursos avançados específicos do dispositivo |

### 12.4 Comandos de Validação em Tempo Real

Comandos de fluxo de eventos online (protocolo REON):

| Código | Direção | Propósito | Acionado Por |
|--------|---------|-----------|--------------|
| 000+0 | Dispositivo → Servidor | Solicitação de acesso | Entrada de cartão/biometria/teclado |
| 00+1 | Servidor → Dispositivo | Liberar ambas direções | Lógica de validação do servidor |
| 00+5 | Servidor → Dispositivo | Liberar entrada | Lógica de validação do servidor |
| 00+6 | Servidor → Dispositivo | Liberar saída | Lógica de validação do servidor |
| 00+30 | Servidor → Dispositivo | Negar acesso | Lógica de validação do servidor |
| 000+80 | Dispositivo → Servidor | Aguardando giro | Após acesso liberado |
| 000+81 | Dispositivo → Servidor | Giro completado | Usuário passou |
| 000+82 | Dispositivo → Servidor | Tempo limite de giro | Usuário não passou |

### 12.5 Fases de Implementação para Emulador Turnkey

**Fase 1: Protocolo Central** (Completo)
- [x] Parsing de mensagem (STX, ID, PROTOCOLO, COMANDO, DADOS, ETX, CHECKSUM)
- [x] Construção de mensagem com cálculo de checksum XOR
- [x] Conexões de servidor/cliente TCP
- [x] Reconhecimento e categorização de código de comando

**Fase 2: Configuração e Status** (Em Progresso)
- [ ] EC/RC - Get/set de configuração
- [ ] RQ - Consultas de status do dispositivo (todos os 12 tipos)
- [ ] EH/RH - Sincronização de data/hora
- [ ] Banco de dados offline para validação local

**Fase 3: Gerenciamento de Usuários e Acesso**
- [ ] EE/RE - Configuração de empregador
- [ ] EU/RU - Operações CRUD de usuários
- [ ] ECAR/RCAR - Operações CRUD de cartões
- [ ] RR - Recuperação de log de acesso (todos os 5 modos de filtro)
- [ ] Marcação de evento (coletado vs não coletado)

**Fase 4: Recursos Avançados**
- [ ] EGA/RGA/ECGA/RCGA - Grupos de acesso e associações
- [ ] EPER/RPER/EHOR/RHOR - Controle de acesso baseado em tempo
- [ ] EFER/RFER - Gerenciamento de feriados
- [ ] EACI/RACI - Acionamentos de relé agendados
- [ ] EMSG/RMSG - Customização de mensagem de display

**Fase 5: Biometria** (Opcional)
- [ ] ED/RD - Tratamento de template biométrico
- [ ] Integração de SDK Control iD
- [ ] Integração de SDK Digital Persona
- [ ] Implementação de leitor biométrico mock

**Fase 6: Validação em Tempo Real** (Prioridade)
- [ ] REON+000+0 - Tratamento de solicitação de acesso
- [ ] REON+00+1/5/6 - Respostas de liberação de acesso com controle de relé
- [ ] REON+00+30 - Resposta de negação de acesso com logging
- [ ] REON+000+80/81/82 - Rastreamento de evento de giro
- [ ] Gerenciamento de timeout (fallback online/offline)

**Fase 7: Processamento em Lote**
- [ ] Execução de comando baseada em arquivo
- [ ] Comandos especiais (*pause*, *sleep*)
- [ ] Modo de loop com interrupção ESC
- [ ] Rastreamento de progresso e relatório de erro

### 12.6 Estratégia de Teste

**Testes Unitários**:
- Parsing/construção de mensagem para cada tipo de comando
- Validação de cálculo de checksum
- Tratamento de separador de campo
- Validação de formato de data/hora

**Testes de Integração**:
- Fluxo de acesso completo (solicitação → liberação → giro → log)
- Fallback offline quando servidor indisponível
- Resolução de permissão de grupo de acesso
- Aplicação de regra de acesso baseada em tempo

**Testes de Hardware** (requer dispositivos físicos):
- Integração de leitor RFID ACR122U
- Temporização de ativação de relé
- Renderização de mensagem de display
- Tratamento de entrada de teclado

**Testes de Conformidade de Protocolo**:
- Compatibilidade com versões de firmware Primme Acesso
- Validação de subconjunto de comando Argos
- Suporte de comando básico Primme SF

## 13. Implementação do Emulador

### 13.1 Estados da Catraca

```python
class EstadoCatraca(Enum):
    IDLE = 0
    AGUARDANDO_VALIDACAO = 1
    AGUARDANDO_GIRO = 2
    GIRO_EM_PROGRESSO = 3
    GIRO_COMPLETADO = 4
    TIMEOUT = 5
    BLOQUEADA = 6
```

### 13.2 Fluxograma de Estados

```
[IDLE] → Cartão Apresentado → [AGUARDANDO_VALIDACAO]
          ↓
    Validação OK?
    Sim → [AGUARDANDO_GIRO] → Timeout → [TIMEOUT] → [IDLE]
                ↓                             ↓
          Giro Iniciado              Liberação Manual
                ↓                             ↓
      [GIRO_EM_PROGRESSO]            [AGUARDANDO_GIRO]
                ↓
        [GIRO_COMPLETADO] → [IDLE]

    Não → [BLOQUEADA] → Timeout → [IDLE]
```

### 13.3 Tempos Limite Importantes

| Evento | Tempo Padrão | Configurável |
|--------|--------------|--------------|
| Resposta Online | 3000ms | Sim (500-10000ms) |
| Aguardando Giro | 5s | Sim (via comando) |
| Modo Offline | 60s | Sim (2-600s) |
| Anti-passback | 0min | Sim (0-999999min) |

### 13.4 Validações Críticas

1. **Formato do Cartão**: 3-20 caracteres ASCII
2. **Formato de Data/Hora**: dd/mm/aaaa hh:mm:ss
3. **Direção**: Valores válidos 0, 1, 2
4. **Tipo de Leitora**: 1=RFID, 5=Biometria
5. **Checksum**: Calcular e validar em todas as mensagens

## 14. Testes e Validação

### 14.1 Cenários de Teste Obrigatórios

#### Fluxo Normal
1. Apresentação de cartão válido
2. Liberação pelo software
3. Giro completo
4. Registro do evento

#### Fluxo com Tempo Limite
1. Apresentação de cartão válido
2. Liberação pelo software
3. Não realização do giro
4. Tempo limite e retorno ao estado inicial

#### Fluxo com Negação
1. Apresentação de cartão inválido
2. Negação pelo software
3. Bloqueio mantido
4. Registro do evento negado

#### Modo Offline
1. Falha na comunicação com servidor
2. Validação local
3. Armazenamento de eventos
4. Sincronização posterior

### 14.2 Validações de Protocolo

- [ ] Formato de mensagem correto
- [ ] Separadores nos lugares corretos
- [ ] Campos obrigatórios presentes
- [ ] Tipos de dados corretos
- [ ] Intervalos de valores respeitados
- [ ] Codificação ASCII mantida
- [ ] Checksum calculado corretamente

## 15. Resolução de Problemas

### 15.1 Problemas Comuns

| Problema | Causa Provável | Solução |
|----------|---------------|---------|
| Catraca não responde | ID incorreto | Verificar configuração NR_EQUIP |
| Tempo limite constante | TIMEOUT_ON muito baixo | Ajustar para 3000-5000ms |
| Giro não detectado | Problema no sensor | Verificar código 81 após giro |
| Cartão não lido | Formato incorreto | Validar número do cartão |
| Display sem mensagem | MSG_DISPLAY vazia | Configurar mensagem padrão |

### 15.2 Logs Recomendados

```
[TIMESTAMP] [NIVEL] [COMPONENTE] [MENSAGEM]
2024-01-15 10:23:45 INFO CATRACA_01 Cartão apresentado: 12651543
2024-01-15 10:23:45 DEBUG PROTOCOLO TX: 01+REON+00+0]12651543]...
2024-01-15 10:23:46 DEBUG PROTOCOLO RX: 01+REON+00+1]5]Acesso liberado]
2024-01-15 10:23:46 INFO CATRACA_01 Acesso liberado, aguardando giro
2024-01-15 10:23:48 INFO CATRACA_01 Giro completado, direção: ENTRADA
```

## 16. Referências e Versões

### 16.1 Versões do Protocolo

| Versão | Data | Principais Mudanças |
|--------|------|-------------------|
| 1.0.0.7 | - | Comando Quantidade e Status |
| 1.0.0.8 | - | Suporte teclado, leitoras numeradas |
| 1.0.0.9 | - | Modo de cadastro automático |
| 1.0.0.10 | - | Resposta online melhorada |
| 1.0.0.23 | - | Versão estável Primme Acesso |
| 8.0.0.50 | - | Versão atual com todos recursos |

### 16.2 Compatibilidade

- **Primme Acesso**: Protocolo completo
- **Argos**: Protocolo simplificado (sem grupos, períodos, etc.)
- **Primme SF**: Protocolo básico (apenas validação online)

## 17. Notas Finais

Este documento representa a consolidação completa da documentação do protocolo Henry, aprimorada com descobertas do emulador cliente Java oficial do fabricante. O protocolo suporta três classes de dispositivos com conjuntos de recursos variados:

**Primme Acesso** (Protocolo Completo):
- Todos os comandos documentados nas seções 1-11
- Grupos de acesso, períodos de tempo, horários, feriados
- Acionamentos de relé e customização de display
- Recursos biométricos avançados
- Monitoramento de status abrangente

**Argos** (Protocolo Simplificado):
- Validação central (REON)
- Configuração básica (EC/RC)
- Cartões e biometria (ECAR/ED)
- Logs de acesso (RR)
- Consultas de status (RQ)

**Primme SF** (Protocolo Básico):
- Apenas validação online
- Configuração mínima
- Gerenciamento de cartões
- Suporte biométrico básico

### Diretrizes de Implementação

Para implementação do emulador, é essencial:

1. **Seguir Rigorosamente os Formatos de Mensagem**: Separadores de campo, formatos de data e cálculos de checksum devem corresponder exatamente.

2. **Respeitar Timeouts Configurados**: Padrão de 3000ms para validação online, faixa configurável de 500-10000ms.

3. **Manter Compatibilidade**: Implementar detecção de recursos para suportar diferentes modelos de dispositivos graciosamente.

4. **Tratar Casos Extremos**: Fallbacks de timeout, abandono de giro, falhas de rede, transições de modo offline.

5. **Implementar Paginação**: Para operações de dados em massa (cartões, usuários, logs) para evitar exaustão de memória.

6. **Validar Entrada**: Números de cartão (3-20 caracteres), datas (dd/mm/aaaa HH:MM:SS), direções (0/1/2), etc.

7. **Testar Extensivamente**: Use o recurso de processamento de arquivo em lote para criar suítes de teste abrangentes.

### Abordagem de Desenvolvimento

A implementação deve ser modular para suportar diferentes níveis de funcionalidade:

- **Módulo Central**: Parsing de mensagem, checksum, TCP/IP (todos dispositivos)
- **Módulo de Validação**: Lógica de validação online/offline (todos dispositivos)
- **Módulo de Configuração**: Tratamento de parâmetros EC/RC (todos dispositivos)
- **Módulo de Gerenciamento de Usuários**: Operações EU/ECAR (Primme Acesso, Argos)
- **Módulo de Controle de Acesso**: Grupos, períodos, horários (apenas Primme Acesso)
- **Módulo Biométrico**: Tratamento de template ED/RD (SDKs específicos do dispositivo)

Esta arquitetura em camadas permite:
- Alternância fácil de recursos baseada no tipo de dispositivo
- Separação limpa de responsabilidades
- Testabilidade com implementações mock
- Extensibilidade futura para novos comandos

O emulador Turnkey visa fornecer uma implementação completa e fiel do protocolo Henry para fins de desenvolvimento, teste e integração.