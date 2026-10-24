# Changelog

Each release of furb is listed here, the newest first. release-please writes the entry of each release from the
titles of the pull requests that it holds, as [the template of a pull request](.github/pull_request_template.md) says.

## [0.3.0](https://github.com/uael/furb/compare/v0.2.0...v0.3.0) (2026-10-24)


### Features

* **compaction:** Compact a chain in place, so its work goes on ([#375](https://github.com/uael/furb/issues/375)) ([f41fac4](https://github.com/uael/furb/commit/f41fac415a830b2f2852405a3e46a78791adab52))
* **compaction:** Go on from a chain of another record ([#427](https://github.com/uael/furb/issues/427)) ([be61d11](https://github.com/uael/furb/commit/be61d115fd49706b66dc322bb58fe359b164cd5a))
* **compaction:** Let compacted choose what the new chain keeps ([#317](https://github.com/uael/furb/issues/317)) ([e600ee4](https://github.com/uael/furb/commit/e600ee443d688feac4431f940a8322848897a578))
* **conductor:** Change the conductor in one call of patch() ([#379](https://github.com/uael/furb/issues/379)) ([92c2c77](https://github.com/uael/furb/commit/92c2c771d00fdb8b918a9e4a3e9107af493372d0))
* **conductor:** Close each worker with a PR number and a report ([#356](https://github.com/uael/furb/issues/356)) ([3bc7440](https://github.com/uael/furb/commit/3bc7440d349341fb16b3bee06bc96b8dc8c0ed24))
* **conductor:** Queue an item for a slow job and for a flaky test ([#413](https://github.com/uael/furb/issues/413)) ([c1785bf](https://github.com/uael/furb/commit/c1785bf2ae32d9a12ff6d73fc1f4248004c8846c))
* **converge:** Converge the TUI toward the words of the owner ([#414](https://github.com/uael/furb/issues/414)) ([0c56923](https://github.com/uael/furb/commit/0c56923cd79ac7164e5b02a2165ab14a67fd9d08))
* **crate:** Go on from a checkpoint of a life on any build ([#415](https://github.com/uael/furb/issues/415)) ([7530bcf](https://github.com/uael/furb/commit/7530bcfd7e97dee6cd528026ffb51bf59a41039b))
* **crate:** Replay a record on any build of the version that made it ([#380](https://github.com/uael/furb/issues/380)) ([7538615](https://github.com/uael/furb/commit/7538615802b60726ec07c4fa286cb3e29ca61bf3))
* **crate:** Run a record on the build of its version from a store ([#384](https://github.com/uael/furb/issues/384)) ([d9acecf](https://github.com/uael/furb/commit/d9acecf285ee4b252b54ab90db9aaa9b7325aa6d))
* **crate:** Run each word that ran at a checkpoint again from its top ([#431](https://github.com/uael/furb/issues/431)) ([78a0798](https://github.com/uael/furb/commit/78a0798036bcadb039c8e0ba3d712a8d131d10da))
* **crate:** Upgrade an older record at a stand through a checkpoint ([#419](https://github.com/uael/furb/issues/419)) ([8f11fb9](https://github.com/uael/furb/commit/8f11fb97547aaa6cb2beb621bde7fe865a788bf8))
* **engine:** Let a prefix replace the transcript of its chain ([#371](https://github.com/uael/furb/issues/371)) ([6e81637](https://github.com/uael/furb/commit/6e816375be4a7d4424d0e15b9c54d937867545db))
* **engine:** Tell the number of lines of a read that shows none ([#349](https://github.com/uael/furb/issues/349)) ([2ce25e5](https://github.com/uael/furb/commit/2ce25e5eb049a8214de18dabb1e392c6a316b366))
* **extensions:** Reload a changed extension, and keep its state ([#386](https://github.com/uael/furb/issues/386)) ([eaf4048](https://github.com/uael/furb/commit/eaf40483c4a3f8a4e6ee8dbe60b7a534e982622b))
* **gui:** Pause, wake, cancel, take back and rewind in the app ([#363](https://github.com/uael/furb/issues/363)) ([db0a9b6](https://github.com/uael/furb/commit/db0a9b6111b270cdf4686eb726f6888f5cf64b05))
* **gui:** Run a life in the app on the host of the TUI ([#361](https://github.com/uael/furb/issues/361)) ([22d0dfc](https://github.com/uael/furb/commit/22d0dfca97c3fce83a74320cc21344bc9bbb0c99))
* **provider:** End a reply at the first statement of bad python ([#440](https://github.com/uael/furb/issues/440)) ([90a10c0](https://github.com/uael/furb/commit/90a10c0cd05ae909a3094f9c3419066cd332dcc8))
* **release:** Release furb to PyPI, npm and crates.io at one version ([#327](https://github.com/uael/furb/issues/327)) ([10f7b95](https://github.com/uael/furb/commit/10f7b959637424c5af5a4cbef402992c24751fed))
* **tui:** Hold each life in a host that outlives the window ([#381](https://github.com/uael/furb/issues/381)) ([5f30327](https://github.com/uael/furb/commit/5f303278371568d7a46f0e897cb07639ef05ef68))
* **tui:** Offer the values of a question as a list, with Other ([#390](https://github.com/uael/furb/issues/390)) ([95f5a9d](https://github.com/uael/furb/commit/95f5a9decab2ee3bbf0f17b8e6ae16d4c2d1a83d))
* **tui:** Serve a life to other operators, to read or to drive it ([#430](https://github.com/uael/furb/issues/430)) ([ce7febc](https://github.com/uael/furb/commit/ce7febcbc319ae4af9e9e24edb2d5adbd898d43b))
* **tui:** Show a calm palette that takes the colors of the terminal ([#388](https://github.com/uael/furb/issues/388)) ([9d39695](https://github.com/uael/furb/commit/9d396950a6a651e5187e537b87b7812283dab48c))
* **tui:** Take over a host of another build from a new window ([#423](https://github.com/uael/furb/issues/423)) ([a7964eb](https://github.com/uael/furb/commit/a7964eba53797c5dc2e20c3c82883d3142e99b69))


### Fixes

* **ci:** Read the outputs of release-please only when it succeeded ([#374](https://github.com/uael/furb/issues/374)) ([f830498](https://github.com/uael/furb/commit/f830498befb8243abe577a7694131d3d4d432370))
* **cli:** Name the work that an earlier life left pending at a run ([#398](https://github.com/uael/furb/issues/398)) ([6df6b84](https://github.com/uael/furb/commit/6df6b84b08f0acd0d3a66b9a285f10b1907c3bbd))
* **conductor:** Clear the git variables of a commit in every test ([#436](https://github.com/uael/furb/issues/436)) ([f214ac2](https://github.com/uael/furb/commit/f214ac2e91a2e6012783382f3ba88e7577be2dbb))
* **conductor:** Keep workers on sonnet or opus and in one chain ([#418](https://github.com/uael/furb/issues/418)) ([611ad49](https://github.com/uael/furb/commit/611ad4940a751da263e1343219e31cbce5e927f5))
* **conductor:** Land and heal open PRs before new work ([#396](https://github.com/uael/furb/issues/396)) ([550cd6f](https://github.com/uael/furb/commit/550cd6f7f128ca043db9abf9e9d103cf9c8b05d8))
* **conductor:** Read main as red only when its gates fail ([#373](https://github.com/uael/furb/issues/373)) ([480d4fd](https://github.com/uael/furb/commit/480d4fdf0f426be153a7b60a4fdbce7018eff3c4))
* **conductor:** Run the git of the tests with no variable of a commit ([#417](https://github.com/uael/furb/issues/417)) ([a74f24b](https://github.com/uael/furb/commit/a74f24b38c1f0704b95883adf7adc0573b625d90))
* **conductor:** Tell an idle PR only when it can land ([#364](https://github.com/uael/furb/issues/364)) ([47d9014](https://github.com/uael/furb/commit/47d9014eb45bf433be871234083ccf725897a79f))
* **conductor:** Tick a verified box with no line of evidence under it ([#377](https://github.com/uael/furb/issues/377)) ([b3c3160](https://github.com/uael/furb/commit/b3c3160c6b37e259cd97d212d576d4e0a0a31809))
* **crate:** Bound the depth and the size of a value that a host gets ([#399](https://github.com/uael/furb/issues/399)) ([638573d](https://github.com/uael/furb/commit/638573d88908613255552cccd20310e891ade2a7))
* **crate:** Carry no answer of an act that a checkpoint drops ([#432](https://github.com/uael/furb/issues/432)) ([c365ffb](https://github.com/uael/furb/commit/c365ffbc632cfb049c2a319e48c01871278657af))
* **crate:** Keep in the log each act that no later life makes again ([#369](https://github.com/uael/furb/issues/369)) ([f464d29](https://github.com/uael/furb/commit/f464d297215046672b40e2a8e94e0776e4fa0470))
* **crate:** Take no snapshot of a life that said nothing new ([#401](https://github.com/uael/furb/issues/401)) ([2c47c93](https://github.com/uael/furb/commit/2c47c93d898f2201d2c0ff14ce5fbb489cf044c4))
* **crate:** Tell an ear the past of a restore by the rule of the bus ([#400](https://github.com/uael/furb/issues/400)) ([11aa0d6](https://github.com/uael/furb/commit/11aa0d6d0b5d82c9089c9ee080f2adcf1f8b5308))
* **docs:** Make each link of a package README work on its registry ([#351](https://github.com/uael/furb/issues/351)) ([3643009](https://github.com/uael/furb/commit/364300987cd45cadee5f04a9c771bd504da283b0))
* **doctrine:** Fix a refused word in its place, through the door ([#391](https://github.com/uael/furb/issues/391)) ([1db3595](https://github.com/uael/furb/commit/1db35950241ed21fc7a0f3c12645c35597efa83e))
* **engine:** Fit the engine under the wall again, 22 tokens fewer ([#382](https://github.com/uael/furb/issues/382)) ([fbab98e](https://github.com/uael/furb/commit/fbab98ee546a554ae7a2c03a6c245ebb8b95f94c))
* **engine:** Refuse a control of an id that names no act ([#350](https://github.com/uael/furb/issues/350)) ([de2a411](https://github.com/uael/furb/commit/de2a41100875d2597d8b9e51e7c4c89b99506bc1))
* **engine:** Refuse a word that the chain cannot gate or run ([#389](https://github.com/uael/furb/issues/389)) ([1503046](https://github.com/uael/furb/commit/15030464f4e69cafbe6441afa8fdf5113f97dc8c))
* **engine:** Unquote a told text of any length, so its chain goes on ([#366](https://github.com/uael/furb/issues/366)) ([312ee54](https://github.com/uael/furb/commit/312ee54ddfbb25038e3aab8b8fa8293e08918187))
* **framework:** End each wait on a life once the life ended ([#404](https://github.com/uael/furb/issues/404)) ([84e92c3](https://github.com/uael/furb/commit/84e92c38598cbf6f58fe500f075382f32f06fd09))
* **gui:** End each check of the app, and run it in a macOS gate ([#367](https://github.com/uael/furb/issues/367)) ([1d9396f](https://github.com/uael/furb/commit/1d9396f6fdc0fd6ed7cc3aa7a3dfa916d5dbec96))
* **gui:** Read the web view of a check before the app ends ([#385](https://github.com/uael/furb/issues/385)) ([95dd4f2](https://github.com/uael/furb/commit/95dd4f22d918e296d45b822c9cdb42fefd2d4756))
* **provider:** Fill each place of the address of a provider once ([#403](https://github.com/uael/furb/issues/403)) ([f6125ab](https://github.com/uael/furb/commit/f6125abcd69d649a4fc2b9b4d5f42531061adf45))
* **provider:** Wait for the reset of a usage limit and go on ([#421](https://github.com/uael/furb/issues/421)) ([9bf4391](https://github.com/uael/furb/commit/9bf43919bf361ff135383e18a26f08d7d1eec9ed))
* **tui:** Give back the files of a rewind as the journal holds them ([#409](https://github.com/uael/furb/issues/409)) ([6219de4](https://github.com/uael/furb/commit/6219de49afef34844d953714755f2f789da40175))
* **tui:** Hold the retry stream of the footer test through each state ([#378](https://github.com/uael/furb/issues/378)) ([859261f](https://github.com/uael/furb/commit/859261fec9ef09fac3b724bf31fc63c596ef1d1d))
* **tui:** Import the readings of an act from the model of a life ([#405](https://github.com/uael/furb/issues/405)) ([39baa5f](https://github.com/uael/furb/commit/39baa5f246d86aeba38c39bc6242377510cbb3bc))
* **tui:** Keep a session open on a damaged journal and repair it whole ([#429](https://github.com/uael/furb/issues/429)) ([48a825b](https://github.com/uael/furb/commit/48a825b13e41d4aba27b897e42c8263b543dba77))
* **tui:** Make the ground of the README gif transparent ([#441](https://github.com/uael/furb/issues/441)) ([a5e14e7](https://github.com/uael/furb/commit/a5e14e777061e3ff072f3979a66b73b4e4bac81d))
* **tui:** Open a field of a saved view of another form empty ([#407](https://github.com/uael/furb/issues/407)) ([38bacbd](https://github.com/uael/furb/commit/38bacbdeab87687e3140560400cd801f1a8e25c3))
* **tui:** Reach a new host when the one that ends drops the window ([#408](https://github.com/uael/furb/issues/408)) ([92f4653](https://github.com/uael/furb/commit/92f46532af07241d5d8d836a4948c53544c27616))
* **tui:** Wake every paused chain when /wake names none ([#422](https://github.com/uael/furb/issues/422)) ([529fac4](https://github.com/uael/furb/commit/529fac4f454908774b6288be95def4236fa3ca60))
* **typescript:** Give one typed reading of the standing ([#406](https://github.com/uael/furb/issues/406)) ([29f621a](https://github.com/uael/furb/commit/29f621ade01239333d072c79c64618cdeff1534d))
* **typescript:** Keep a snapshot of the life of a Session ([#370](https://github.com/uael/furb/issues/370)) ([e452474](https://github.com/uael/furb/commit/e4524746f7498aa075ae63dd0e075df22459bc1c))
* **web:** Stop a fetch as soon as the page passes its bound ([#402](https://github.com/uael/furb/issues/402)) ([99ae097](https://github.com/uael/furb/commit/99ae0978bb6399a80cbd12100b724b4e35dc67ff))


### Performance

* **engine:** Tell the doctrine before the first prompt to its actor ([#340](https://github.com/uael/furb/issues/340)) ([5605d24](https://github.com/uael/furb/commit/5605d248569de6aa98d5ecb8a89586fc0adcec98))
* **engine:** Unquote in one search, and make 66 tokens of room ([#387](https://github.com/uael/furb/issues/387)) ([44c5a24](https://github.com/uael/furb/commit/44c5a2443ec1fe60ea0acf470d620e1dea7788b0))
* **tui:** Keep the cost of a new fact flat as a life grows ([#392](https://github.com/uael/furb/issues/392)) ([82c52d9](https://github.com/uael/furb/commit/82c52d9b80d850e23aeee1a2cca3b570a7c8a42f))

## 0.1.0 and the steps to 0.2.0

furb 0.1.0 came out on PyPI on 2026-09-19, as one Python package. The work between 0.1.0 and 0.2.0 came before
this changelog, in commits with no type, so this short history gives its main steps, with the pull request of each.

- 2026-09-19, 0.1.0: the contract of the engine, a suite with one test for each sentence of it, the engine itself,
  and a World, a Kernel and a door in Python.
- 2026-09-22: the crate runs the engine in monty, a Python interpreter written in Rust, with a door to Python (#8).
- 2026-09-24: the bindings for TypeScript and the TUI on OpenTUI (#21), then a new design of the TUI (#37).
- 2026-09-26: the engine, derived again from its meanings: every question is an act (#43).
- 2026-09-27: a provider of models in Rust, with the claude command line as a model (#45), and `furb`, the command
  line, which opens the TUI or serves a JSON-RPC (#46).
- 2026-09-28: Rust as the only provider and a small system of extensions (#50), and an engine that the crate loads
  with no preamble (#78).
- 2026-10-20: a scratch directory for each record (#136), the models of a local server (#140), and `furb facts`
  (#174).
- 2026-10-21: the official extensions compaction (#183), ouroboros (#224) and web (#249), a store of the record on
  SQLite (#282), and the suite on Windows (#263).
- 2026-10-22: examples for each door (#253), the log of a life beside its record (#284), a snapshot that opens a life
  with no replay (#288), the doctrine in its own file (#299), a record that names its engine (#301), and wheels for
  Windows (#306).
- 2026-10-23: a retry of a model call that fails for a cause that passes (#324), a check of the setup at the first run
  (#321), the desktop preview that reads records (#325), the audit of a record (#326), and in the TUI three views at
  once (#329), parts of extensions (#330), a review of the changes (#331), files in a rewind (#332), and a mode for a
  screen reader (#334).
