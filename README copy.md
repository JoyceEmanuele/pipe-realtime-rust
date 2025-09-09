# realtime-service

### Tarefas do Realtime (em Rust):
* Mantém registro da última telemetria de cada dispositivo
  - Endpoints que informam o horário e o conteúdo da última telemetria
  - Tarefa que periodicamente salva no disco as últimas telemetrias para conseguir recuperar quando reiniciar o serviço
* Notificações verificadas em tempo real:
  - Horário de funcionamento do compressor (DAC)
  - Temperatura do ambiente (DUT)
  - Nível de CO2 (DUT)
  - Usa uma API interna para buscar as notificações cadastradas
