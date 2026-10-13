# Uplink C++ → Rust porting guide

This is the rulebook used for porting the Uplink Developer CD source (the C++ in `source/` and `lib/` on the CD, not included here) to Rust.
Every porter follows it so that files written in parallel fit together.
When this guide and your instinct disagree, follow the guide. When the guide is silent, stay
as close to the C++ as Rust allows and leave a `// PORT:` comment explaining any deviation.

The reference C++ is the `source/` and `lib/` folders of the Uplink Developer CD (not included in this repository).

## 1. Workspace layout

```
rust/
  crates/engine/   platform layer (DONE, do not edit unless told): archive, redshirt, gl, image, text, sound, music
  crates/uplink/   the game
    src/lib.rs     module tree root
    src/main.rs    binary entry -> uplink::uplink_main::main()
    src/ctx.rs     the Ctx god-object passed everywhere (replaces C++ globals `app` and `game`)
    src/tosser.rs  DArray<T> and BTree<T> (C++ tosser DArray / BTree). LList -> Vec
    src/objref.rs  ObjRef / ScreenRef / DataBankOwner: serialisable references to world objects
    src/eclipse/   port of lib/eclipse (buttons + animations)
    src/vanbakel/  port of lib/vanbakel (task manager core)
    src/uplink_main.rs  port of source/uplink.cpp
    src/<dir>/<file>.rs  one Rust file per C++ .cpp/.h pair, same directory structure as source/
```

**File mapping:** `source/a/b/c.cpp` + `source/a/b/c.h` → `crates/uplink/src/a/b/c.rs`
(module path `crate::a::b::c`). Each directory has a `mod.rs` that only declares submodules
(already generated — do not add or remove modules without being asked).
Special cases: `source/uplink.cpp` → `src/uplink_main.rs`; `lib/eclipse/*` → `src/eclipse/`;
`lib/vanbakel/*` → `src/vanbakel/`; `source/app/dos2unix.*`, `resizex.cpp`, `serialise.*`,
`uplinkobject.*` are NOT ported (see §6).

**Name mapping:**
| C++ | Rust |
|---|---|
| class `PasswordScreen` | struct `PasswordScreen` (keep the exact class name, even `BBSScreen`, `IPLookup`) |
| method `GetComputerScreen` | `get_computer_screen` (snake_case; `SetTYPE`→`set_type`, `GetOBJECTID` not ported) |
| field `TYPE` / `STATUS` | `type_` / `status` (lowercase; `type_` because `type` is a keyword) |
| field `ip`, `maintitle` | same name |
| `#define SIZE_X 64` (string size limits) | drop it (Strings are unbounded) unless used in logic |
| `#define SCREEN_PASSWORDSCREEN 5` | `pub const SCREEN_PASSWORDSCREEN: i32 = 5;` in the module of the header that defined it |
| `#define` function-like macros | `pub fn` |
| static member function | associated fn (`impl X { pub fn foo(..) }`) |
| static member/global variable | field on the owning struct, or a field on `Ctx` (ask) — never `static mut` |

Every constant/type/function keeps living in the Rust module that corresponds to the C++ header that
declared it. To find where something lives, find its C++ header (see `docs/classmap.txt`).

## 2. Types

| C++ | Rust |
|---|---|
| `int` | `i32` (keep i32 even for sizes/indices that are compared with -1) |
| `float` / `double` | `f32` / `f64` |
| `bool` | `bool` |
| `char name[N]`, `char *` owned string | `String` |
| `char *` that may be NULL | `Option<String>` |
| `const char *` parameter | `&str` (`Option<&str>` if NULL is passed) |
| `LList<T*>` / `LList<char*>` | `Vec<T>` / `Vec<String>` |
| `DArray<T*>` | `crate::tosser::DArray<T>` (sparse: slots can be empty; keeps indices stable) |
| `DArray<int>` | `Vec<i32>` unless holes matter |
| `BTree<T*>` keyed by name/ip | `crate::tosser::BTree<T>` (duplicate keys, pre-order iteration like the C++) |
| `BTree<char*>` | `crate::tosser::BTree<String>` |
| owned `T*` member (may be NULL) | `Option<Box<T>>` (or `Option<T>`) |
| owned `T` member | `T` |
| non-owning pointer to a world object | a key (`String` name/ip) or `ObjRef`/`ScreenRef` (see `objref.rs`) |
| `Date` | `crate::world::date::Date` (Copy-less struct, `.clone()` it) |
| `Image *` | `Option<Rc<uplink_engine::image::Image>>` (shared) or `Image` (owned) |
| `time_t` | `i64` (unix seconds) |

All world-model structs (everything under `world/`, plus `Options`, `Game` data you save) derive
`#[derive(Debug, Clone, Default, Serialize, Deserialize)]` (`serde::{Serialize, Deserialize}`).
Use `#[serde(default)]` on structs. Interface/UI structs derive `Default` (and `Debug` if easy).

## 3. Inheritance

Pick the pattern listed for the hierarchy. Do not invent another.

1. **ComputerScreen subclasses** (GenericScreen, MessageScreen, PasswordScreen, MenuScreen, DialogScreen,
   UserIDScreen, LogScreen, BBSScreen, LinksScreen, SharesListScreen, CypherScreen, HighSecurityScreen,
   DisconnectedScreen): enum-of-structs. In `world/computer/computerscreen/computerscreen.rs`:
   ```rust
   pub struct ComputerScreenBase { pub maintitle: String, pub subtitle: String, pub computer: String }
   pub enum ComputerScreen { GenericScreen(GenericScreen), MessageScreen(MessageScreen), ... }
   impl ComputerScreen { pub fn base(&self) -> &ComputerScreenBase; pub fn base_mut(..); pub fn screen_type_name(&self) -> &'static str }
   ```
   Each subclass struct has `pub base: ComputerScreenBase` as its first field plus its own fields.
   Base-class methods (`SetMainTitle`, `GetComputer`) are implemented on `ComputerScreenBase`.
2. **UplinkEvent subclasses** (ArrestEvent, AttemptMissionEvent, BankRobberyEvent, ChangeGatewayEvent,
   InstallHardwareEvent, NotificationEvent, RunPlotSceneEvent, SeizeGatewayEvent, ShotByFedsEvent,
   WarningEvent): same enum-of-structs pattern: `UplinkEventBase { pub rundate: Date }`,
   `enum UplinkEvent { ArrestEvent(ArrestEvent), ... }`, every subclass has `pub base: UplinkEventBase`.
   Virtual methods dispatch with a `match` in `uplinkevent.rs`:
   `pub fn run(self, g: &mut Ctx)`, `pub fn run_warning(&self, g: &mut Ctx)`, `get_short_string(&self) -> String`, `get_long_string`.
3. **Computer → BankComputer, LanComputer**: one struct `Computer` with all base fields plus
   `pub kind: ComputerKind` where `enum ComputerKind { Plain, BankComputer(BankComputer), LanComputer(LanComputer) }`
   and `BankComputer`/`LanComputer` hold only the subclass's extra fields. Helpers on Computer:
   `bank()`, `bank_mut()`, `lan()`, `lan_mut()` returning `Option<&..>`. Subclass methods are implemented
   on the subclass struct when they only touch subclass data, otherwise as `Computer` methods in the subclass's file.
   Overridden virtuals (`Update`, ...) dispatch by matching on `kind` inside `Computer`'s method.
4. **VLocation → VLocationSpecial**: `VLocation` has `pub special: Option<VLocationSpecial>` (the extra fields).
5. **Company → CompanyUplink**: `Company` has `pub uplink: Option<Box<CompanyUplink>>` (the extra fields + methods on CompanyUplink).
6. **Person → Agent → Player**: `Person` holds the Person fields plus `pub agent: Option<Box<Agent>>` where
   `Agent` holds only the Agent-specific fields (links, codes, missions, handle). Agent *methods* are
   implemented as `impl Person` methods in `world/agent.rs` (they call `self.agent_mut()`).
   `Player` is `pub struct Player { pub person: Person, pub gateway: Gateway, shares..., score_* }` with
   `impl Deref<Target = Person> for Player` + `DerefMut`, so `player.name`, `player.give_link(ip)` work.
   The world stores the player in `World::player: Option<Box<Player>>` (NOT in `people`);
   `World::get_person("PLAYER")` / `get_person_mut` return the player's Person. NPC agents live in
   `World::people` as `Person`s with `agent: Some(..)`.
7. **Interface screens** (LocalInterfaceScreen, RemoteInterfaceScreen, MainMenuScreen, NetworkScreen
   subclasses, and HUDInterface): trait objects. Each screen is a struct implementing
   `crate::interface::interfacescreen::InterfaceScreen` (see that file) and invoking `impl_as_any!()`.
   Remote screens store `pub cs: Option<ScreenRef>` instead of `ComputerScreen *cs`.
8. **UplinkTask subclasses** (all of `interface/taskmanager/*` except taskmanager itself): trait objects
   implementing `crate::interface::taskmanager::uplinktask::UplinkTask`; each struct has
   `pub base: UplinkTaskBase` (pid, version, followmouse, targetobject: ObjRef, targetstring, targetint).
9. Anything else small (WorldMapInterfaceObject/Label, Option/ColourOption, ...): a struct with an
   optional/kind field, porter's choice, documented with `// PORT:`.

## 4. Globals and the `Ctx` object

The C++ reached everything through the globals `app` and `game`. In Rust everything is owned by
`crate::ctx::Ctx`, passed as `g: &mut Ctx` (or `g: &Ctx` for read-only code):

```rust
pub struct Ctx {
    pub app: App,          // app->...      (App owns options, network, mainmenu)
    pub game: Game,        // game->...     (Game owns world, interface (ui), view)
    pub ecl: Eclipse,      // all Ecl* calls
    pub svb: Vanbakel,     // all Svb* calls (task list)
    ...
}
```
| C++ | Rust |
|---|---|
| `game->GetWorld()` | `g.world()` / `g.world_mut()` |
| `game->GetWorld()->GetPlayer()` | `g.player()` / `g.player_mut()` |
| `game->GetInterface()` | `g.interface()` / `g.interface_mut()` |
| `game->GetInterface()->GetLocalInterface()` | `g.interface_mut().li` (field) |
| `game->GetInterface()->GetRemoteInterface()` | `g.interface_mut().ri` |
| `game->GetInterface()->GetTaskManager()` | `g.interface_mut().tm` |
| `game->IsRunning()` | `g.game.is_running()` |
| `app->GetOptions()` | `g.app.options` |
| `app->GetMainMenu()` | `g.app.mainmenu` |
| `app->GetNetwork()` | `g.app.network` |
| `game->GetWorld()->date` | `g.world().date` |

**Borrowing.** `g.world_mut()` borrows all of `g`. When you need two parts at once, borrow the
fields directly (`let w = g.game.world.as_deref_mut().unwrap(); let ecl = &mut g.ecl;`), or copy what you
need out first (`let ip = comp.ip.clone();`) and re-borrow. Cloning a `String` to satisfy the borrow
checker is always fine. Never use `unsafe`, `Rc<RefCell<..>>` for world objects, or raw pointers.

**Methods that touch globals.** A C++ method whose body (directly or through calls it makes) uses
`game`/`app`/`Ecl*`/`Svb*`/`Sg*` cannot be `&mut self` on an object that lives inside `Ctx`
(the borrow checker would see two `&mut`). Port it as an associated function that takes the context
and the object's *key* instead of `self`:
```rust
// C++: void Computer::CheckForSecurityBreaches ()   (uses game->...)
impl Computer {
    pub fn check_for_security_breaches(g: &mut Ctx, computer_name: &str) { ... }
}
// C++: VLocation *Person::GetLocalHost ()  (read-only lookup through the world)
impl Person {
    pub fn get_local_host<'a>(&self, world: &'a World) -> Option<&'a VLocation> { ... }
}
```
Keys: VLocation → `ip`; Computer → `name`; Company → `name`; Person → `name` ("PLAYER" for the player);
ComputerScreen → `ScreenRef { computer, index }`; Mission → index in its owner's list.
Methods that only touch `self` stay `&self` / `&mut self`. Read-only world lookups take `&World`.
Interface screens and tasks are the exception: they are trait objects held outside the world
and get `(&mut self, g: &mut Ctx)` (see §5).

**Random numbers:** use `crate::world::generator::numbergenerator::NumberGenerator::random_number(range)`
etc. (ported from numbergenerator.cpp); for plain `rand()` use `crate::app::miscutils::rand_i32()`.

**Build flags** (`app/globals.h`): this port builds the FULLGAME configuration on Linux.
`#ifdef WIN32` → take the non-Windows branch. `DEMOGAME`, `TESTGAME`, `DOCLABRELEASE`, `WAREZRELEASE`,
`CODECARD_ENABLED`, `DEBUGLOG_ENABLED` → `pub const X: bool` in `app/globals.rs`, ported as
`if globals::DEMOGAME { .. }` so nothing is lost. `CHEATMODES_ENABLED` → `globals::cheatmodes_enabled()`
(runtime, env var `UPLINK_CHEATS=1`). `_DEBUG` → `cfg!(debug_assertions)`.

**Errors:** `UplinkAssert(x)` → `debug_assert!(x)` or `.expect("...")` when unwrapping;
`UplinkAbort(msg)` → `panic!(...)`; `UplinkWarning(msg)` → `eprintln!(...)`. `printf` → `println!`.
`sprintf/strcpy/strcat` → `format!`/`String` ops. `strcmp(a,b)==0` → `a == b`.
`atoi` → `.trim().parse().unwrap_or(0)`. `sscanf` → split/parse by hand.

## 5. UI layer

### Eclipse (buttons)  — `crate::eclipse`
| C++ | Rust |
|---|---|
| `EclRegisterButton(x,y,w,h,caption,tooltip,name)` | `g.ecl.register_button(x, y, w, h, caption, tooltip, name)` |
| `EclRegisterButton(x,y,w,h,caption,name)` | `g.ecl.register_button(x, y, w, h, caption, "", name)` |
| `EclRegisterButtonCallbacks(name, draw, up, down, move)` | `g.ecl.register_button_callbacks(name, Some(draw), Some(up), None, None)` |
| `EclRegisterButtonCallback(name, up)` | `g.ecl.register_button_callback(name, up)` |
| `EclGetButton(name)` (may be NULL) | `g.ecl.get_button(name)` → `Option<&Button>`; `get_button_mut` |
| `EclRemoveButton(name)` | `g.ecl.remove_button(name)` |
| `EclRegisterCaptionChange(name, text, ms, cb)` | `g.ecl.register_caption_change(name, text, ms, Some(cb))` |
| `EclRegisterMovement(name, x, y, ms, cb)` | `g.ecl.register_movement(name, x, y, ms, MOVE_STRAIGHTLINE, None)` |
| any other `EclFoo(...)` | `g.ecl.foo(...)` (snake_case, same args; see `eclipse/mod.rs`) |
| `button->caption` etc. | `button.caption` |
| `button->SetCaption(c)` | `g.ecl.get_button_mut(name).unwrap().set_caption(c)` |

Callback signatures (plain `fn` items, not closures):
```rust
pub type DrawFn      = fn(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool);
pub type MouseFn     = fn(g: &mut Ctx, button: &Button);           // mouseup / mousedown / mousemove
pub type AnimCallback = fn(g: &mut Ctx);                           // animation finished
```
`button` is a snapshot clone: to change the real button use `g.ecl.get_button_mut(&button.name)`.
C++ static member callbacks (e.g. `PasswordScreenInterface::PasswordClick`) become associated fns
with these signatures.

### Drawing — `uplink_engine::{gl, text, image}`
`glBegin(GL_QUADS)` → `gl::begin(gl::QUADS)`, `glVertex2i` → `gl::vertex2i`, `glColor4f` → `gl::color4f`,
`glColor3ub` → `gl::color3ub`, `glLineWidth` → `gl::line_width`, `glEnable(GL_SCISSOR_TEST)` → `gl::enable(gl::SCISSOR_TEST)`,
`glScissor` → `gl::scissor` (same GL bottom-left arguments; the screen height is `g.app.options` screen height,
use `crate::app::opengl::screen_h()` helpers if the C++ used `app->GetOptions()->GetOptionValue("graphics_screenheight")`),
`glLineStipple` → `gl::line_stipple`, `glEnd` → `gl::end`. `glPushAttrib/glPopAttrib/glMatrixMode/glLoadIdentity/glHint/
glBlendFunc` → drop. `glPushMatrix+glTranslatef / glPopMatrix` → `gl::push_translate(dx,dy)` / `gl::pop_translate()`.
`GciDrawText(x,y,t,STYLE)` → `text::draw_text_styled(x, y, t, STYLE)`; `GciDrawText(x,y,t)` → `text::draw_text_default`;
`GciTextWidth` → `text::text_width(t, STYLE)` / `text::text_width_default(t)`; font ids `text::HELVETICA_10` etc.
`Image` → `uplink_engine::image::Image` (`load(path)`, `draw`, `draw_blend`, `scale`, `set_alpha`, `width`, ...).
`SgPlaySound(RsArchiveFileOpen("sounds/x.wav"), "sounds/x.wav", sync)` → `sound::play_sound("sounds/x.wav", None, sync)`.
`RsArchiveFileOpen(path)` → just use the archive path string with `uplink_engine::archive::read(path)` /
`Image::load(path)` / `archive::read_text(path)`. Graphics paths: the C++ helper `button_assignbitmap(name, "close.tif")`
prefixes `graphics/` itself; keep that behaviour in `app/opengl_interface.rs`.
`SetColour("Name")` → `crate::app::opengl_interface::set_colour(g, "Name")` (theme colours live in Options).

### Interface screens — `crate::interface::interfacescreen`
```rust
pub trait InterfaceScreen: Any {
    fn create(&mut self, g: &mut Ctx) {}
    fn remove(&mut self, g: &mut Ctx) {}
    fn update(&mut self, g: &mut Ctx) {}
    fn is_visible(&self, g: &Ctx) -> bool { false }
    fn screen_id(&self) -> i32 { 0 }
    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool { false }
    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool { false }
    fn set_computer_screen(&mut self, cs: ScreenRef) {}  // remote screens: called before create()
    fn as_any(&self) -> &dyn Any; fn as_any_mut(&mut self) -> &mut dyn Any;   // impl_as_any!()
}
```
Hosts (LocalInterface, RemoteInterface, MainMenu, the HUD, NetworkClient) keep their current screen in a
`ScreenSlot`. While a screen method runs, the screen is *taken out of its slot*. Consequences:
- Inside a screen's own methods use `self`, never re-fetch "the current screen" from `g`.
- Static callbacks (button clicks) fetch the screen with
  `g.interface_mut().ri.screen.get_mut::<PasswordScreenInterface>()` (returns `Option`; if `None`, just return).
- Switching screens (`RunScreen`) from inside a screen method is automatically deferred by the host until
  the method returns (the C++ object was deleted at that point anyway, so code after RunScreen must not use self).

### Tasks (running software) — `crate::interface::taskmanager::uplinktask::UplinkTask` + `crate::vanbakel`
Trait methods mirror vanbakel's `Task`: `initialise`, `tick(n)`, `create_interface`, `remove_interface`,
`show_interface`, `is_interface_visible`, `set_target(ObjRef, &str, i32)`, `move_to`, plus `base()/base_mut()`.
`SvbRemoveTask(pid)` → `Vanbakel::remove_task(g, pid)` (deferred automatically if that task is running).
`GetPID()` → `self.base().pid`. Task targets that were `UplinkObject *` are `ObjRef` (see objref.rs).

## 6. Not ported / replaced
- `UplinkObject::Load/Save/Print/GetID/GetOBJECTID`, `LoadID/SaveID`, all of `serialise.cpp`, `CreateUplinkObject`:
  replaced by serde derives. Saving = `serde_json` of the world (see `app/app.rs`). `Print()` → `#[derive(Debug)]`.
- `tosser` → std collections + `crate::tosser::DArray` / `crate::tosser::BTree`.
- `slasher` (memory leak checker), `dos2unix`, `resizex` (window resize hacks), code-card copy protection screens are
  ported but unreachable when `CODECARD_ENABLED` is false.
- `Update()` from UplinkObject: keep where the class defines one.

## 7. Workflow rules for porters
- Only edit the files you were assigned. If you need something from another module that does not exist,
  do NOT create it there: write the call as you expect it to look, and list it in your final report.
- Keep comments that explain game logic; drop include lists and banner comments.
- No `unwrap()` on lookups that can legitimately fail in the C++ (`if (!comp) return;` → `let Some(comp) = .. else { return };`).
- Build: `rust/tools/cargo-gate check` (a wrapper that limits parallel builds; plain `cargo` may be killed for
  memory). Never run `cargo clean`. Never run cargo with more than `-j4`.
- Do not use `todo!()` in a body you were asked to port; port all of it. If something is truly impossible,
  write the closest working code and a `// PORT-TODO:` comment.
