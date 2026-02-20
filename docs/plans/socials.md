Esto es tan importante como el código en sí. Voy a darte una estrategia realista, no el típico "postea en redes"
  genérico.                                                                                                         
                                                                                                                    
  ---                                                                                                               
  La regla base: primero producto, luego comunidad                                                                  
                                                                                                                    
  No hay atajo. Si lanzas antes de que Phase 1 esté sólida, quemas la oportunidad. Un dev que prueba Sentinel y     
  encuentra falsos positivos o crashes no vuelve. El product-market fit tiene que existir antes de cualquier        
  campaña.                                                                                                          
                                                                                                                    
  Timeline honesto: lanzar cuando Phase 1 y Phase 2 estén terminadas.                                               
                                                              
  ---
  Canales por orden de impacto

  1. GitHub — tu base de operaciones

  El README es tu landing page. La mayoría de devs decide en 30 segundos si van a probar una herramienta.

  Lo que tiene que tener el README:
  - Un GIF o video de 30 segundos mostrando Sentinel detectando dead code en tiempo real mientras Claude Code
  escribe
  - Una sola frase clara: "The quality layer that runs alongside your AI coding tool"
  - Instalación en 1 comando (curl | sh o brew install)
  - Comparativa rápida: qué hace ESLint/SonarQube vs qué hace Sentinel que ellos no

  Lo que dispara stars:
  - Buenas Issues con labels (good first issue, help wanted)
  - CHANGELOG detallado
  - Releases con notas claras
  - Responder PRs y Issues en menos de 24h en los primeros 3 meses

  ---
  2. Hacker News — el mayor multiplicador

  Un buen "Show HN" puede traer 500-2000 stars en 48 horas si el timing y el framing son correctos.

  Cómo hacerlo bien:
  - Postear un martes o miércoles entre 9-11am ET
  - Título: Show HN: Sentinel – quality guardian for AI-generated code (Rust)
  - El primer comentario tuyo tiene que explicar el por qué exististe, no el qué
  - Preparar el servidor para el tráfico (GitHub aguanta, pero tu docs site no)
  - No postear hasta tener al menos 50 GitHub stars de early adopters reales

  El ángulo que funciona en HN: el problema de "AI code rot" es nuevo y técnicamente interesante. Un post bien
  argumentado sobre por qué el código generado por AI degrada la calidad de los proyectos puede viralizarse por sí
  solo, sin ni siquiera mencionar Sentinel directamente.

  ---
  3. Twitter/X — build in public

  Esta es la estrategia con mejor ROI para herramientas de developer. No es sobre seguidores, es sobre
  conversaciones.

  Qué postear:
  Semana 1: El problema (sin mencionar Sentinel)
  "Llevamos 6 meses usando Claude Code en producción.
  El código funciona. Los tests pasan. Pero tenemos
  340 funciones declaradas que nunca se llaman.
  Esto es lo que estamos haciendo al respecto 🧵"

  Semana 2-4: El proceso de construcción
  GIFs del análisis AST, capturas de dead code detectado,
  números concretos (X funciones detectadas, Y falsos positivos)

  Lanzamiento: demo en video real

  A quién mencionar / con quién interactuar:
  - Devs que hablan de Claude Code, Cursor, Copilot
  - Threads sobre "problemas con AI coding tools"
  - Responder con valor antes de pedir nada

  ---
  4. Comunidades específicas — más efectivo que los influencers

  ┌───────────────────────┬─────────────────────┬───────────────────────────────┐
  │       Comunidad       │        Dónde        │            Cuándo             │
  ├───────────────────────┼─────────────────────┼───────────────────────────────┤
  │ NestJS Discord        │ discord.gg/nestjs   │ Phase 1 completa              │
  ├───────────────────────┼─────────────────────┼───────────────────────────────┤
  │ Rust Users Forum      │ users.rust-lang.org │ Phase 1 (ángulo técnico)      │
  ├───────────────────────┼─────────────────────┼───────────────────────────────┤
  │ r/programming         │ reddit.com          │ Show HN primero, luego Reddit │
  ├───────────────────────┼─────────────────────┼───────────────────────────────┤
  │ r/node / r/typescript │ reddit.com          │ Cuando soporte TS sea sólido  │
  ├───────────────────────┼─────────────────────┼───────────────────────────────┤
  │ TypeScript Discord    │ Por invitación      │ Phase 2                       │
  └───────────────────────┴─────────────────────┴───────────────────────────────┘

  Regla: nunca entres a una comunidad solo a postear tu link. Participa 2-3 semanas antes, luego comparte cuando sea
   genuinamente relevante.

  ---
  5. Influencers — el canal más sobreestimado

  La mayoría no responde DMs de proyectos desconocidos. Los que sí responden, cobran o exigen exclusividad. Pero hay
   una forma que funciona:

  Lo que sí funciona:
  - Resolver un problema que ellos tienen públicamente. Si Fireship o Theo postean un video sobre problemas con AI
  coding tools, envíales un mensaje diciendo "construí algo para esto, aquí hay un video demo de 2 minutos, úsalo si
   quieres". Sin pedirte nada.
  - Contribuir a sus proyectos. Si tienen repos open source, contribuye. Luego tienes contexto para mencionarles
  Sentinel.
  - Crear contenido técnico que ellos quieran compartir. Un post de "Analizé 10 proyectos reales usando Claude Code
  y encontré esto" con datos reales es algo que los influencers comparten porque les ahorra trabajo.

  Influencers donde tiene sentido (en orden de alineación):
  1. Devs que hablan de AI coding tools (muchos en X, creciendo rápido)
  2. Rust YouTubers (nicho pero muy comprometido)
  3. Canales de TypeScript/NestJS
  4. Theo (t3.gg) — habla mucho de calidad de código y TypeScript

  ---
  La estrategia completa en timeline

  Hoy → Phase 1:     Silencio total. Construir.

  Phase 1 lista:     Conseguir 10 early adopters de confianza.
                     Feedback real. Iterar.

  Phase 2 lista:     README perfecto. Demo GIF.
                     Postear Show HN.
                     Build in public en Twitter empieza.

  +30 días:          Primer artículo técnico:
                     "El problema del AI code rot y cómo resolverlo"
                     (Dev.to + Hashnode + LinkedIn)

  +60 días:          Contactar influencers con producto probado
                     y testimonios reales.

  +90 días:          Product Hunt launch (necesitas
                     hunters conocidos para esto).

  ---
  El contenido que más funciona para herramientas dev

  Demos en video cortos (30-60 seg): mostrar el terminal detectando dead code en tiempo real mientras Claude Code
  escribe. Sin narración, solo el tool en acción. Estos se comparten solos.

  El "número sorprendente": "Analizé mi propio proyecto con Sentinel y encontré 47 funciones que nunca se llaman.
  Todas generadas por AI en el último mes." Eso es compartible.

  La comparativa honesta: "Qué hace ESLint, qué hace SonarQube, qué hace Sentinel que los otros no pueden." Sin
  exagerar, sin atacar.

  ---
  Resumen: el canal más importante es GitHub bien hecho + un Show HN en el momento correcto. Lo demás amplifica,
  pero esos dos son el núcleo. Y todo esto funciona solo si el producto realmente resuelve el problema mejor que lo
  gratuito existente.