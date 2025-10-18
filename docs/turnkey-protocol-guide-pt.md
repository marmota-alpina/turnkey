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

### 9.1 Coleta de Registros

#### Todos os Registros
```
<SB><XXXX><II>+ER+00+T]QUANTIDADE]INDICE_INICIAL<CS><EB>
```

#### Apenas Não Coletados
```
<SB><XXXX><II>+ER+00+C]QUANTIDADE]INDICE_INICIAL<CS><EB>
```

#### Filtrado por Data/Hora
```
<SB><XXXX><II>+ER+00+D]QUANTIDADE]DATA_INICIAL]DATA_FINAL<CS><EB>
```

### 9.2 Confirmação de Coleta

```
<SB><XXXX><II>+ER+00+QTD_COLETADOS+INDICES]<CS><EB>
```

## 10. Consulta de Quantidades e Status

### 10.1 Parâmetros Disponíveis

| Parâmetro | Descrição | Valores |
|-----------|-----------|---------|
| D | Quantidade de digitais | 0-10000 |
| U | Quantidade de usuários | 0-50000 |
| R | Quantidade de registros | 0-999999999 |
| RNC | Registros não coletados | 0-999999999 |
| RNCO | Registros offline não coletados | 0-999999999 |
| C | Quantidade de cartões | 0-999999999 |
| TP | Status bloqueado | A/D |
| TD | Máximo de biometrias suportadas | 300+ |

### 10.2 Exemplo de Uso

```
// Solicitar quantidade de usuários
<SB><XXXX><II>+RQ+00+U<CS><EB>

// Resposta: 35 usuários
<SB><XXXX><II>+RQ+00+U]35<CS><EB>
```

## 11. Mensagens Personalizadas

### 11.1 Estrutura de Envio

```
<SB><XXXX><II>+EMSG+00+MODO_ENT_L1[MSG_ENT_L1[MODO_ENT_L2[MSG_ENT_L2[MODO_SAI_L1[MSG_SAI_L1[MODO_SAI_L2[MSG_SAI_L2<CS><EB>
```

Onde:
- `MODO_*`: Modo de operação da mensagem
- `MSG_*`: Texto da mensagem
- `ENT`: Entrada
- `SAI`: Saída
- `L1/L2`: Linha 1 ou 2 do display

## 12. Implementação do Emulador

### 12.1 Estados da Catraca

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

### 12.2 Fluxograma de Estados

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

### 12.3 Tempos Limite Importantes

| Evento | Tempo Padrão | Configurável |
|--------|--------------|--------------|
| Resposta Online | 3000ms | Sim (500-10000ms) |
| Aguardando Giro | 5s | Sim (via comando) |
| Modo Offline | 60s | Sim (2-600s) |
| Anti-passback | 0min | Sim (0-999999min) |

### 12.4 Validações Críticas

1. **Formato do Cartão**: 3-20 caracteres ASCII
2. **Formato de Data/Hora**: dd/mm/aaaa hh:mm:ss
3. **Direção**: Valores válidos 0, 1, 2
4. **Tipo de Leitora**: 1=RFID, 5=Biometria
5. **Checksum**: Calcular e validar em todas as mensagens

## 13. Testes e Validação

### 13.1 Cenários de Teste Obrigatórios

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

### 13.2 Validações de Protocolo

- [ ] Formato de mensagem correto
- [ ] Separadores nos lugares corretos
- [ ] Campos obrigatórios presentes
- [ ] Tipos de dados corretos
- [ ] Intervalos de valores respeitados
- [ ] Codificação ASCII mantida
- [ ] Checksum calculado corretamente

## 14. Resolução de Problemas

### 14.1 Problemas Comuns

| Problema | Causa Provável | Solução |
|----------|---------------|---------|
| Catraca não responde | ID incorreto | Verificar configuração NR_EQUIP |
| Tempo limite constante | TIMEOUT_ON muito baixo | Ajustar para 3000-5000ms |
| Giro não detectado | Problema no sensor | Verificar código 81 após giro |
| Cartão não lido | Formato incorreto | Validar número do cartão |
| Display sem mensagem | MSG_DISPLAY vazia | Configurar mensagem padrão |

### 14.2 Logs Recomendados

```
[TIMESTAMP] [NIVEL] [COMPONENTE] [MENSAGEM]
2024-01-15 10:23:45 INFO CATRACA_01 Cartão apresentado: 12651543
2024-01-15 10:23:45 DEBUG PROTOCOLO TX: 01+REON+00+0]12651543]...
2024-01-15 10:23:46 DEBUG PROTOCOLO RX: 01+REON+00+1]5]Acesso liberado]
2024-01-15 10:23:46 INFO CATRACA_01 Acesso liberado, aguardando giro
2024-01-15 10:23:48 INFO CATRACA_01 Giro completado, direção: ENTRADA
```

## 15. Referências e Versões

### 15.1 Versões do Protocolo

| Versão | Data | Principais Mudanças |
|--------|------|-------------------|
| 1.0.0.7 | - | Comando Quantidade e Status |
| 1.0.0.8 | - | Suporte teclado, leitoras numeradas |
| 1.0.0.9 | - | Modo de cadastro automático |
| 1.0.0.10 | - | Resposta online melhorada |
| 1.0.0.23 | - | Versão estável Primme Acesso |
| 8.0.0.50 | - | Versão atual com todos recursos |

### 15.2 Compatibilidade

- **Primme Acesso**: Protocolo completo
- **Argos**: Protocolo simplificado (sem grupos, períodos, etc.)
- **Primme SF**: Protocolo básico (apenas validação online)

## Notas Finais

Este documento representa a consolidação completa da documentação obtida sobre o protocolo Henry. Para implementação do emulador, é essencial seguir rigorosamente os formatos de mensagem, respeitar os tempos limite configurados e manter a compatibilidade com as diferentes versões de firmware dos equipamentos.

A implementação deve ser modular para suportar os diferentes níveis de funcionalidade entre Primme Acesso, Argos e Primme SF, mantendo um núcleo comum de comunicação e validação.