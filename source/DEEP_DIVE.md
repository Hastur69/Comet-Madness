# Crazy Comets (Commodore 64, 1985) — A Deep Dive

*Research notes that informed COMET CRAZE. All descriptions are paraphrased; sources are listed at the end.*

---

## 1. The basics

| | |
|---|---|
| **Platform** | Commodore 64 |
| **Year** | 1985 (Rob Hubbard's own work list dates it to November 1985) |
| **Publisher** | Martech (later reissued on the Ricochet budget label as RC 003) |
| **Design & code** | Simon Nicol |
| **Music & sound effects** | Rob Hubbard |
| **Genre** | Single-screen, multidirectional shoot-'em-up |
| **Players** | 1–2 (alternating turns) |

## 2. Origins: a Mad Planets clone in all but name

Crazy Comets is an unofficial home version of **Mad Planets**, a 1983 Gottlieb arcade game designed and programmed by **Kan Yabumoto**, with art by Jeff Lee and sound by David Thiel. The resemblance was not subtle: Martech's logo used the same lettering treatment as the arcade game, simply swapping "Mad Planets" for "Crazy Comets".

The C64 lineage goes back further than the commercial release. Simon Nicol and John Griffin had already written a C64 version under the *Mad Planets* name that was never officially published; it was later reworked, given a Rob Hubbard soundtrack and sold by Martech as Crazy Comets. Contemporary trivia (via Commodore Format) notes that Nicol was still a schoolboy, and wrote most of the game during his summer holidays.

### What Mad Planets brought to the table
- Planets appear and **grow** at the start of a wave. Kill them before they reach full size and they never sprout moons — clearing a wave this way earns a big bonus.
- A grown planet is **shielded by moons**. Once those moons are shot away (or hurled at the player), the planet becomes enraged and charges.
- **Comets** orbit the screen and get faster the longer they survive.
- Periodic **bonus rounds** featured astronauts to rescue by flying over them, with comet values climbing in steps of 100 up to 1,000.
- The cabinet used a flight-style joystick to move, a rotary knob to aim, and a trigger to fire.

## 3. The story (such as it is)

The universe has spun out of control and is collapsing into chaos; matter is turning into energy in a runaway chain reaction. Your job is to blast everything that comes at you. Zzap!64 noted dryly that there is no objective beyond staying alive as long as possible.

## 4. How it plays

**A fake-3D single screen — and a ship that never turns.** Although everything is 2D, the screen pretends to have depth: the player's ship is seen from behind, pointing *into* the screen. Unlike the arcade original (where a rotary knob aimed the ship), the C64 ship **cannot rotate: it always faces and fires straight up**, so you line up shots by flying underneath your targets. It moves freely in all directions with a little inertia and momentum. Enemies first appear as tiny dots in the distance and swell as they approach — and crucially, **they cannot hurt you until they are close enough**, although you can shoot them at any time.

**How the planets move.** Planets hurtle in from the sides of the screen at speed, small at first, and grow into big spinning bodies. They can't steer freely: they change direction only by bouncing off the screen edges or by following a curve — yet big ones still home in on you like missiles. Gamebase64 describes two kinds: **Worlds** (land and sea — copies of the Earth) that simply explode once hit enough, and **Novas** (pockmarked with craters) that begin glowing angrily when shot and soon go wild, homing in very fast.

**Grow, go crazy, go nova.** The orbs grow the longer they live. Small ones die in a few shots; a neglected one becomes a huge, many-hit planet. Hit a grown planet enough and it starts **flashing and moves much faster** — harder to dodge, but worth more. Left alone, objects escalate into **novas/supernovas** surrounded by **satellites**, which keep coming until the nova is destroyed and serve as a steady trickle of points.

**Homing hazards.** Much of the game is spent dodging objects with a homing-missile style of movement while trying to shoot them down.

**Mini comets.** From around the end of round two, small comets with glowing tails whizz across the screen. They die from a single hit but are fast and hard to track. (A known bug made them flicker between their own shape and the nova shape.)

**Capsules / tubes.** On later levels, lost space capsules (the C64-Wiki describes them as white tube-shaped objects) drift through. You collect them by **touching** them for 200 points; shooting one risks catching you in its explosion.

**Scoring.** Every hit scores a little; bigger kills score more, with supernovas (plus an attacking satellite) at the top. An **extra life arrives every 10,000 points**, and a **Perfect Round bonus of 1,000** rewards clearing a round quickly while everything is still small. Comet colours change from level to level, and you cannot fire during level transitions.

**Controls.** Holding fire gives auto-fire, so no special joystick is needed; there are no weapon upgrades.

**Rough edges.** The C64-Wiki is blunt about unfair moments: mini comets can spawn right on top of the ship in intermediate rounds, and two full-size objects on screen at once is close to a death sentence. Collisions, on the other hand, are forgiving — just clipping a nova's orbit with the ship's nose won't necessarily kill you. Its strategy advice is the heart of the game: **kill them while they're small**, fire constantly, and if something reaches full size, avoid it and clear the rest first.

## 5. The music — the real star

Rob Hubbard composed **two music tracks plus all of the sound effects**. The sound design is cleverly integrated: effects borrow a SID channel from the in-game music when needed, and the C64-Wiki singles this out as working far better than in other games of the era.

- Hubbard has named Crazy Comets among his personal favourites (alongside Sanxion, Kentilla, W.A.R. and International Karate), and the C64-Wiki reports he considers it one of his finest SID works.
- Wikipedia's summary of his work lists the tune as **inspired by New Order and funk**.
- A modern retrospective describes roughly four minutes of music full of funky drums and bass and lead lines that build steadily toward the final section; there is also a separate high-score tune.
- German magazine *Happy Computer* went as far as saying the music alone justified the price.
- The tune still has a life of its own: it appears on the *C64 Greatest SIDs: The Art of Rob Hubbard* vinyl, and Remix64 continues to list new Crazy Comets remixes (including one dated April 2026), plus Chris Abbott's "Mad Planets – Comets '77" homage.

## 6. Reception

- **Zzap!64 (issue 8)** welcomed it as a refreshingly uncomplicated, no-frills shoot-'em-up, praising some superb graphical touches and hours of mindless blasting at a tempting price, while noting limited variety.
- **MobyGames** lists an average critic score of about 70%.
- A modern **Mastertronic.co.uk** review gives 7.5/10: an unofficial Mad Planets that becomes very fast and frantic, with responsive controls and an iconic soundtrack — though it wished for a music/SFX toggle.
- The **C64-Wiki** verdict is harsher: the soundtrack shines, the gameplay was left behind.

## 7. Legacy

- **Mega Apocalypse (1987)** — Nicol's sequel (Zzap!64 Sizzler, issue 30). It added two-player simultaneous play, pick-ups collected at the start of each wave (rotation, homing missiles, speed, shields, extra lives), title art by Bob Stevenson, Nicol's own sound effects and speech samples, and a Rob Hubbard remix of the Crazy Comets theme. It was ported to the BBC Micro, Amstrad CPC and ZX Spectrum.
- **Apocalypse 2** — a third instalment for the SNES, also by Nicol, which was never released.

## 8. How COMET CRAZE maps the original

| 1985 original | COMET CRAZE (2026) |
|---|---|
| Ship seen from behind, can't rotate, always fires straight up | Same: rear-view ship, banks as it slides, twin cannons fire straight up; shots shrink as they fly "into" the screen |
| Free 8-way movement with some inertia | Same, tuned to be snappy |
| Planets enter from the sides at speed, start small and grow | Same |
| Planets only change course by bouncing or curving, big ones home in | Same: curvature-based steering that bends harder toward you as planets grow |
| Small/distant planets can't hurt you | Same: harmless (and drawn fogged, "behind" you) until 30% grown |
| Bigger = more hits | Hits needed scale with size; points rise from 100 to 600 |
| Worlds: explode when hit enough | Earth-textured Worlds |
| Novas: glow angrily when shot, then go wild and home in fast | Cratered Novas with a 1.8 s fuse, then a fast homing rampage (worth triple) |
| Satellites | Fully grown planets gain a satellite that breaks off to attack |
| Small comets from round 2 that chase you | Same: they streak about, then hunt you after a few seconds; one hit kills |
| Rockets/capsules: fly over for 200, shooting does nothing | Same |
| Extra life every 10,000 | Same |
| Perfect Round bonus | +1,000 × round if no planet grew large and no life was lost |
| Colours change per level | Each round shifts the palette |
| No firing in transitions | Same |
| Unfair spawns on top of the ship | Fixed: planets enter from the sides; comets from the edges |
| — | Modern additions: 3D-shaded planets, bloom/CRT, beat-synced visuals, small combo multiplier, attract-mode demo, saved high scores |

## Sources

- [Crazy Comets — Wikipedia](https://en.wikipedia.org/wiki/Crazy_Comets)
- [Crazy Comets — C64-Wiki](https://www.c64-wiki.com/wiki/Crazy_Comets)
- [Crazy Comets — MobyGames](https://www.mobygames.com/game/15398)
- [Crazy Comets — VideoGameGeek](https://videogamegeek.com/videogame/72119/crazy-comets)
- [Zzap!64 #8 review — everygamegoing.com](https://everygamegoing.com/larticle/Crazy-Comets-000/27866)
- [Game Review: Crazy Comets — mastertronic.co.uk](https://mastertronic.co.uk/game-review-crazy-comets-commodore-64-ricochet/)
- [Mad Planets — Wikipedia](https://en.wikipedia.org/wiki/Mad_Planets)
- [Rob Hubbard — Wikipedia](https://en.wikipedia.org/wiki/Rob_Hubbard)
- [Rob Hubbard — C64-Wiki](https://c64-wiki.com/wiki/Rob_Hubbard)
- [Mega Apocalypse — Wikipedia](https://en.wikipedia.org/wiki/Mega_Apocalypse)
- [Mega-Apocalypse — C64-Wiki](https://www.c64-wiki.com/wiki/Mega_Apocalypse)
- [Zzap!64 #30 Mega-Apocalypse review — everygamegoing.com](https://everygamegoing.com/larticle/Mega-Apocalypse-000/28488)
- [Rob Hubbard remixes — Remix64](https://www.remix64.com/composer/rob-hubbard/)
- [Crazy Comets — Gamebase64 Game of the Week](https://gb64.com/oldsite/gameofweek/8/gotw_crazycomets.htm)
