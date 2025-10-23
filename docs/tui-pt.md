# Documenta a interfaçe de usuário com o emulador Turnkey

## Estrutura geral

A tela é dividida em **duas grandes colunas**:

```
+---------------------------------------------------+-----------------------+
|                 COLUNA ESQUERDA                   |    COLUNA DIREITA     |
|             (emulador da catraca)                 |        (logs)         |
+---------------------------------------------------+-----------------------+
```

### Coluna Esquerda — “Emulador da Catraca”

A coluna esquerda ocupa cerca de **70% da largura** do terminal.
Dentro dela, temos **duas seções verticais**:

1. **Display (parte superior)**
2. **Teclado (parte inferior)**

#### DISPLAY (simula o visor LCD da catraca)

Ocupa a parte superior da coluna esquerda — algo como 30% da altura total.

```
+---------------------------------------------------+
|                [DISPLAY / VISOR LCD]              |
|  ------------------------------------------------ |
|  Mensagem: DIGITE SEU USUÁRIO                     |
|  Entrada: [______]                                |
|                                                   |
|  Estado: ONLINE | Modo: MOCK | IP: 192.168.0.10   |
+---------------------------------------------------+
```

**Funções e comportamento:**

* Mostra mensagens dinâmicas de status (ex: “Acesso liberado”, “Senha incorreta”).
* Mostra um campo de entrada (`input_buffer`) representando o que o usuário digitou.
* Pode exibir informações de status do emulador (modo, IP, etc).

---

#### TECLADO (simulação física)

Logo abaixo do display vem o teclado numérico (cerca de 70% da altura restante).

```
+---------------------------------------------------+
|                   [TECLADO]                       |
|                                                   |
|   ┌───┬───┬───┐                                   |
|   │ 1 │ 2 │ 3 │                                   |
|   ├───┼───┼───┤                                   |
|   │ 4 │ 5 │ 6 │                                   |
|   ├───┼───┼───┤                                   |
|   │ 7 │ 8 │ 9 │                                   |
|   ├───┼───┼───┤                                   |
|   │ * │ 0 │ # │                                   |
|   └───┴───┴───┘                                   |
|                                                   |
|   [ENTER] [CANCELAR] [LIMPAR]                     |
+---------------------------------------------------+
```

**Funções e comportamento:**

* Cada tecla reage a eventos de input capturados com `crossterm`.
* Pode haver destaque visual para a tecla pressionada.
* “ENTER”, “CANCELAR” e “LIMPAR” podem ser mapeados para Enter, Esc e Backspace.

---

### Coluna Direita — “Painel de Logs”

A coluna direita ocupa cerca de **30% da largura total**, e toda a **altura** do terminal.
É um painel de rolagem para exibir logs em tempo real (novos eventos do emulador, requisições de controle de acesso, respostas, etc).

```
+-----------------------+
|        LOGS           |
|-----------------------|
| [10:21:14] Init mock  |
| [10:21:15] Socket up  |
| [10:21:16] Conn → OK  |
| [10:21:17] REQ: USER  |
| [10:21:18] RESP: 200  |
| [10:21:19] Acesso OK  |
| ...                   |
| ...                   |
+-----------------------+
```

**Funções e comportamento:**

* Usa um `List` do `ratatui` com `ListItems` contendo as mensagens.
* Pode ter um `Scrollbar` (suporte nativo a partir do `ratatui 0.27`).
* Recebe mensagens de um canal (`mpsc`) do emulador.

---

## Sugestão de estilo visual (para aplicar no `ratatui`)

| Elemento            | Estilo sugerido                                                                        |
| ------------------- | -------------------------------------------------------------------------------------- |
| Display box         | Borda dupla (`Block::default().borders(Borders::ALL).border_type(BorderType::Double)`) |
| Teclado             | Borda simples com fundo cinza                                                          |
| Logs                | Fundo escuro, texto em tons de amarelo ou ciano                                        |
| Teclas pressionadas | Realce com `Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)`           |
| Display ativo       | Fundo azul escuro ou preto, texto verde (imitando LCD)                                 |

---

## Sugestão de UX

* Pressionar `Tab` pode alternar foco entre o **display/input** e o **painel de logs**.
* Pressionar `q` sai do emulador.
* Mostrar mensagens de status (“Conectado”, “Simulando entrada...”) na parte inferior do display.