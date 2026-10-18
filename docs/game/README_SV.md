# GRIN – projektpaket för Claude

Version 1.0 • 18 oktober 2026 • Arbetsnamn: GRIN, inte slutligt beslutat.

Ett eget Rust-spel med inspiration från Snoods spelprincip: skjut geometriska varelser, matcha minst tre och få lösa kluster att falla. Spelets identitet är figurernas personligheter. Ingen siktlinje visas: den figur som kommer att träffas reagerar av rädsla. Den arga är redan konstant förbannad och avslöjar inget. Grannarna är nervösa. Rymmaren försöker dra när han har fri väg, och flyr omedelbart om han också har kontakt med en arg.

## Så använder vi paketet

1. Ge Claude hela ZIP-filen och börja med `CLAUDE_START_HERE.md`.
2. Claude kontrollerar arbetsmiljön och aktuell RustyHusky innan kod ändras.
3. Först skapas en separat spelarbetskopia. RustyHusky förblir en generell mall.
4. Bygg enligt milstolparna. Börja med animationsverkstaden, men bygg inte alla funktioner samtidigt.
5. Speltesta varje milstolpe. Dokumentera avvikelser och justera balansvärden efter riktiga tester.

## Vad är beslutat?

Kärnidén, figurerna, rädslan som sikthjälp, den arga med explosion, nervösa grannar, rymmaren samt konfigurerbar bredd/storlek kommer från samtalet. Exakta tider, poäng, rutnätsregler och explosionsradie är genomarbetade **föreslagna startregler**, inte tidigare användarbeslut. De får implementeras som tydligt dokumenterade prototypvärden och sedan utvärderas. Se `docs/01_DECISIONS.md`.

## Innehåll

| Dokument | Syfte |
| --- | --- |
| `CLAUDE_START_HERE.md` | Färdig startinstruktion och ordning för implementation. |
| `AGENTS_GAME_APPENDIX.md` | Spelspecifika agentregler att slå ihop med mallens regler. |
| `docs/01_DECISIONS.md` | Beslutsstatus, omfattning och öppna frågor. |
| `docs/02_GAME_RULES.md` | Regelbok med entydig ordning och specialfall. |
| `docs/03_TECHNICAL_DESIGN.md` | Rutnät, skott, data, tidssteg och RustyHusky-integration. |
| `docs/04_CHARACTERS_ANIMATION.md` | Figurer, animationer, ljud och animationsverkstad. |
| `docs/05_CONFIGURATION_CONTENT.md` | Bredd, zoom, inställningar, banformat och balans. |
| `docs/06_IMPLEMENTATION_PLAN.md` | Milstolpar med leveranser och klarkriterier. |
| `docs/07_TEST_ACCEPTANCE.md` | Regeltester, visuella kontroller och plattformar. |
| `docs/08_TEMPLATE_AUDIT.md` | Granskad RustyHusky-version, begränsningar och källor. |

Dokumenten för utveckling är på engelska, enligt RustyHuskys befintliga regler. Speltexter ska finnas på engelska och svenska. Dokumenten är en implementeringsspecifikation; inget spel, ny GitHub-repository eller körd spelverifiering ingår ännu.

## Viktigaste teknikvalen

- Macroquad och RustyHuskys befintliga `Game`-gränssnitt behålls som grund.
- Logik körs utan fönster, med mallens 120 uppdateringar per simulerad sekund.
- Samma skottberäkning används för rädsla och faktisk träff.
- Figurerna ritas proceduralt: kropp, ansikte och rörelse. Inga obligatoriska spritesheets.
- Sex grannar i ett förskjutet rutnät; cirkulära kollisionsytor även för fyrkanter och romber.
- En speltransaktion avgör explosioner och ras. Animationerna visar resultatet utan att ändra det.
- Inspelning, paus, språk och ljud återanvänds. Förlusthantering behöver en liten generell utökning.

Inga Snood-bilder, ljud, namn på figurer eller banor kopieras. Vi skapar egna uttryck och eget innehåll.
