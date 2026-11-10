# Community builds: sources and credit

These builds are credited to their creators and included as test cases. If you created one of them and want it removed from this repository, open an issue at https://github.com/DBAggie/Exiled-Studio/issues (or contact the maintainers) and we will remove it.

Removing a build also removes its entry in `../fixtures.json` and any divergence-register entry or test that names it (ADR-007 Amendment 1).

## What these files are
Path of Building (PoB) build files published by the creators below, used only as test fixtures for the PoB oracle (`tools/oracle/`, Spike C) and later for the successor's calculations. They are not shown, sold, or distributed as content in the application. The basis for keeping them in this repository is ADR-007 Amendment 1 (`docs/adr/007-corpus-and-coverage.md`).

Each file is the build's PoB export code, decoded (URL-safe base64, then zlib) to XML. The only change: the attributes `lastAccountHash` and `lastCharacterHash` (account identifiers) are removed where present; nothing else is changed. "Downloaded SHA-256" is the decoded export; "SHA-256" is the committed file, also pinned in `../fixtures.json`. All were retrieved on 2026-11-10.

## Terms of the sources (as found on 2026-11-10)
- **Maxroll** (maxroll.gg): the guides link to the Ziff Davis terms of use (https://www.ziffdavis.com/terms-of-use). They allow downloading and copying content for personal, non-commercial use, and otherwise forbid modifying, publishing, reproducing, or distributing the content without permission, and retrieving it with automated tools. Keeping these builds here does not fit those terms; the maintainer accepted that for test use on 2026-11-10 (ADR-007 Amendment 1), with credit and removal on request. No consent has been asked for or given yet; asking the creators is the preferred path and will be recorded here.
- **pohx.net**: the site says it "was created and is maintained by Jadeon77 along with contributions from the PoE and Pohx communities" and is "free to use". It states no terms of use and no license. The PoB codes are hosted on pobb.in, which states no terms either.

The builds were fetched with a few manual requests: Maxroll's guide pages, its PoB links (`https://maxroll.gg/poe/pob/<id>`, served from `https://planners.maxroll.gg/profiles/load/poe/<id>`), and pobb.in's raw codes (`https://pobb.in/<id>/raw`). No fetching is part of the project's code or CI.

## Builds
| File | Creator | Source | Guide updated | PoB link | Tree | Downloaded SHA-256 | SHA-256 | Covers |
|---|---|---|---|---|---|---|---|---|
| `rf-chieftain-league-start.xml` | Pohx (site by Jadeon77) | https://pohx.net/ ("RF Chief League Start (Best)", 3.29) | 2026-07-21 | https://pobb.in/KMJMGblyFcI7 | 3_28 | `251006ac…76e5` | `e03e83c6…8e11` | Righteous Fire; opens on the levelling spec (level 30 Marauder, 10 specs) |
| `rf-chieftain-life-stack.xml` | Pohx (site by Jadeon77) | https://pohx.net/ ("RF Chief Life Stack", 3.29) | not stated | https://pobb.in/XeXHQvT12hSx | 3_28 | `ade5f07b…aa08` | `8fe62732…25ba` | life stacking, regeneration, burning damage over time (level 100) |
| `winter-orb-elementalist.xml` | legi | https://maxroll.gg/poe/build-guides/winter-orb-elementalist-league-starter | 2026-09-17 | https://maxroll.gg/poe/pob/g312000h | 3_29 | `3befee75…1b59` | same | channelled cold spell, exposure; opens on a level 64 spec |
| `penance-brand-ignite-elementalist.xml` | goratha | https://maxroll.gg/poe/build-guides/penance-brand-ignite-elementalist | 2026-09-17 | https://maxroll.gg/poe/pob/cx3c9o0o | 3_28 | `81313e5d…88db` | `48ff126f…d8ca` | brands, ignite |
| `boneshatter-juggernaut.xml` | zizaran | https://maxroll.gg/poe/build-guides/boneshatter-juggernaut-league-starter | 2026-11-03 | https://maxroll.gg/poe/pob/tcjqpe0t | 3_28 | `74a4042b…ae41` | same | attacks, endurance charges, self-damage |
| `kinetic-blast-of-clustering-necromancer.xml` | legi | https://maxroll.gg/poe/build-guides/kinetic-blast-of-clustering-necromancer-league-starter | 2026-09-17 | https://maxroll.gg/poe/pob/xjrdou0t | 3_29 | `29745e99…086f` | same | transfigured gem, wand projectiles |
| `elemental-hit-of-the-spectrum-deadeye.xml` | havoc | https://maxroll.gg/poe/build-guides/elemental-hit-of-the-spectrum-deadeye-league-starter | 2026-07-24 | https://maxroll.gg/poe/pob/49emu0em | 3_28 | `454bcb2a…2202` | same | bow attacks, transfigured gem, Mirage Archer |
| `poisonous-concoction-of-bouncing-pathfinder.xml` | havoc | https://maxroll.gg/poe/build-guides/poisonous-concoction-of-bouncing-pathfinder-league-starter | 2026-09-17 | https://maxroll.gg/poe/pob/cu1qqo03 | 3_28 | `8cead3c0…771e` | same | flask skill, poison |
| `cyclone-shockwave-slayer.xml` | cptngarbage | https://maxroll.gg/poe/build-guides/cyclone-shockwave-slayer-league-starter | 2026-09-17 | https://maxroll.gg/poe/pob/m6hix0r6 | 3_28 | `7b64c6c6…cd7d` | same | channelled melee with a triggered skill, impale |
| `facebreaker-luminary.xml` | shayd | https://maxroll.gg/poe/build-guides/facebreaker-luminary-league-starter | 2026-11-09 | https://maxroll.gg/poe/pob/ob855e0g | 3_29 | `f10c76a7…7d34` | same | Scion Luminary, unarmed Earthshatter slam, bleed |
| `arakaalis-raise-spider-occultist.xml` | shayd | https://maxroll.gg/poe/build-guides/arakaalis-raise-spider-occultist-league-starter | 2026-11-09 | https://maxroll.gg/poe/pob/lnmjza0x | 3_29 | `f73eb896…8e56` | same | minions from a unique (Raise Spiders), poison |

"Guide updated" is the date the guide page gives (Maxroll's page modification date; pohx.net's "Updated"). Full hashes are in `../fixtures.json` (committed) and below (downloaded).

Downloaded SHA-256, full: `rf-chieftain-league-start` 251006ac070ee30490e27c1966c8e2ba94ca241276f92857d6ab784d487176e5; `rf-chieftain-life-stack` ade5f07b6d3a45e34a0ce79f518c099a5325a79f145fb5d1ed99844fde9aaa08; `penance-brand-ignite-elementalist` 81313e5de2ef0c907c69f9910904cc81522a830c18b2fd36f772a524326b88db (the other eight equal their committed hash).

## Agreed but not included
These guides on the agreed list publish no PoB export (their pages embed only a Maxroll planner, with no "Open in PoB" link) as of 2026-11-10: Poison SRS Necromancer, Earthshatter Slayer (the Facebreaker Luminary build covers Earthshatter), Lightning Arrow Deadeye (Elemental Hit of the Spectrum Deadeye covers bows), Explosive Arrow Ballista Champion, Ice Trap Trickster, Rolling Magma Mines Saboteur, Eviscerate Bleed Gladiator. Totems, traps, and mines are therefore not covered by a community build yet.
