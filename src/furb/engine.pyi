from asyncio import Future
from collections.abc import Callable, Generator, Sequence
from contextvars import ContextVar
from dataclasses import dataclass
from string.templatelib import Template
from typing import Final, Literal, Never, Self, overload

WINDOW: Final[int] = 200000
"""WINDOW is the window, in tokens, of a model whose roster entry does not say one."""
OPERATOR: Final[str] = "operator"
"""OPERATOR is the name of the operator in the roster and as an actor."""
TIMEOUT: Final[float] = 600.0
"""TIMEOUT is the timeout, in seconds, of a command that does not say one."""
ROOT: Final[str] = "chain1"
"""ROOT is the name of the root, which every life opens first under that one name, and on which boot stands the life."""
site: Final[ContextVar[str]] = ContextVar("site", default=OPERATOR)
"""Who is speaking is the site, which every fact is said from: the generator while it speaks, the run while it is stepped, the operator otherwise.
Work that an ear begins while it speaks keeps the site of that ear, so what the work says later is said by that ear.
"""

type Fact = tuple[str, str, str, *tuple[object, ...]]
"""A fact is a tuple: its kind, the act it is about, who said it, and its words, deconstructed only by match.
Everything that the engine, the World, the Kernel and the operator say is a fact, and the kind of a fact is its first slot.
A fact says who said it: the rung that made it, the operator outside a rung, or the World or the Kernel.
A fact is on the scope of the act it is about, so the chain it is on is no slot of it.
A control is about the act it is over, a done, a tell and the facts of the World about the act they settle, tell of or come from, and a question about itself.
A verb takes a chain, and the act it makes is on that chain.
A model calls a verb from a rung, and the operator calls the same verb outside a rung.
An act is on the chain that the verb names.
An act is on a chain, and its facts are on its scope.
A chain is on no chain, and its scope is itself.
The World is given the id of the act, and the facts about the act carry the same id.
A prompt that a rung makes on another chain is an act of that chain.
"""
type Question = tuple[str, str, str, str, *tuple[object, ...]]
"""A fact that takes a name of its own when it is said and is answered, which is an act: answered now with a done, or later, with a started now and a done after it.
A question is a fact whose about is its own name and whose first word is the chain it is on.
Every question takes a name, what is answered now as well as what is answered later.
A question is named by its kind, so a fact is a question when the act it is about is named under its kind, which is what question says.
The name of an act is its kind and how many acts of that kind the life has made with it, so the root is chain1, the first command is bash1 and the first prompt is prompt1, and python binds each name as it is.
A read takes its number as a command does, so the first read is read1, whoever made it.
The generator that settles an await of an act from outside a run is named after that act and the task that awaits it, which is the name of no question.
"""
type Saying = tuple[str, str, *tuple[object, ...]]
"""What an ear says: the kind of the fact, the act it is about, and the words, and nothing of who says it, which the bus fills in from whoever is speaking."""
type Ear = Generator[Saying | None, Fact]
"""An ear is any generator of that shape, so the World and the Kernel are ears, and boot takes an ear of the outside under any name it is to hear by.
The World, the ears of the outside that serve the machine, one or many, hears every fact, and every act that no ear before it took: it answers a stand, a clock, a chance, a read and a write of a path nobody of the engine serves, resolved against the working directory of the chain; it takes a command, asks it whether it is merged, feeds it, and ends it at its timeout and at a cancel; it takes a wait and a prompt to the operator, which it shows; and it takes a reply, which it answers with the turn of the model.
The World performs any fact that an extension defines and that the World knows.
The facts that the World says of its own are for the acts that complete later.
An ear speaks by yielding a saying, and the work it began speaks later by say, under the site of the ear that began it.
The chain has the gate read every rung but the ones it wrote itself, and the Kernel begin every rung, by the acts gate and run.
The Kernel takes a run as that run, begins its word in the module of the chain when it hears that it took it, makes a wants as the run when the word waits for an act that is not done, carries the word on at the done of that wants, and says the run done with what the word gave.
The Kernel sets the site to the rung whose word it steps, for as long as it steps it, so what the word says is said by that rung.
"""

type Show = Callable[[list[str]], list[int]]
"""A show is given the lines of a text and gives the numbers of the lines to tell.
A show is any callable of that shape, so a word adds a show by writing one, and span, grep and differs make the shows of the file.
A show is no word of a fact: the verb that was given it keeps it for what it tells, and the ear of the act closes over it, so no record holds one.
"""
type Filter = Callable[[list[Question]], list[Question]]
"""A filter is given the acts of the transcript up to the source of a chain that has one.
A filter says which acts the turns of that chain keep, each with its entries.
A filter is any callable of that shape, so a word adds a filter by writing one, and take makes the filter of the file.
A later life runs the filter again and keeps the same acts, since the word that opened the chain opens it again with it.
A filter is no word of a chain: the verb keeps it for what the chain holds, so no record holds one.
"""

def say(kind: str, about: str, *words: object) -> Fact:
  """The way to say a fact from what is no ear: a word through its verbs, the operator, and the work an ear began, which speaks from its own loop; the fact is said to the living, whole as the bus made it, and given back.
  Every verb of the file, and every verb of an extension, speaks through the two entries of the bus: say for a fact and act for a question.
  A fact reaches the World, the Kernel and the journal only through the bus.
  A rebound verb reaches the World only through the bus.
  Who says it is whoever is speaking, which the site holds, and nothing names another.
  A fact said: it says its kind, the act it is about, who said it and its words, in that order, and nothing else, since the chain it is on is the scope of the act it is about.
  The bus makes every fact whole from what it is given, so nobody holds a fact that is not whole.
  Every generator, the acts first, then those of the engine, then those the life was given from the outside, since the engine settles what it knows before the outside reads it or acts on it, and it asks the outside for nothing it can answer itself.
  Every generator hears every fact it has not heard, in order, until none is left, and each act as act puts it.
  While one speaks nobody hears, and whoever spoke has everyone hear when it is done, so the facts of one speaker stand together in the log, and no one is having everyone hear while another is.
  A done said of a question that has no outcome yet fills its outcome, and a later done of the same question fills nothing.
  """

def act(kind: str, on: str, ear: Callable[[str], Ear] | None, *words: object) -> Act[object]:
  """The way to make a question: it takes a name when it is made, it is put to the ears, its ear, when it has one, is brought to life under that name and given the name, and the name is given back, which is the act to whoever holds it.
  An act is put to every ear of the engine, the acts first, and a busy one hears it once it is done speaking, at its place among what was said.
  Its own ear is born after the ears of the engine heard it, so the acts that ear makes stand after it.
  It is then put to the ears of the outside in turn, until one takes it, and no ear of the outside after that one hears it.
  To take an act is to say a started or a done about it: a done settles it now, and a started says that its done comes later.
  An act that no ear takes and that the record does not hold is refused: the life says it done with a refusal that names its kind.
  An act said: it is begun, and what the call gives is its name, which is awaited for what the act comes to.
  An act said twice under one name is one act, and the second saying brings no second ear and gives the name back.
  Two acts that say the same words under one name are one act.
  The engine refuses an act said from outside a run that names no chain, a chain apart.
  The chain an act is on is the chain named to the call, or the scope of the one that made it when the call names none.
  The ear of an act is given the name of the act and hears every fact said after its birth, and it speaks by yielding a saying.
  An act carries the words of its kind, which are the plain arguments the verb was given, in the order of the verb, and a show or a filter is none of them.
  """

def drive(g: Ear, name: str) -> None:
  """The other way to speak: a generator is brought to life under a name, and from then it hears every fact that is said and says its own.
  One that returns is over and lives no more, which is how a thing that watches for one fact alone is dropped the moment it hears it.
  One that raises while it hears is broken the same way, and what went wrong goes to the one that spoke.
  A generator brought to life under a name and nothing more: it hears from the tip and runs to its first wait, and one born while a fact goes round hears from the next.
  It lives until it returns, and an act that hears nothing more returns at the first fact it hears after its own end.
  A generator that yields a saying is given the fact as the bus said it, and one that yields nothing waits for the next fact said.
  """

def transcript(on: str = "") -> list[Fact]:
  """transcript gives the facts on a chain, each of which the life adds when it is said, so a chain reads at once what it said itself.
  transcript gives a new list at each call, so a word that changes the list it was given changes no transcript.
  The transcript is the whole state of a chain: its module, its program, its working directory and its turns are read off it, and the standing is read off the transcript of the root.
  A name of no chain gives no fact.
  """

def ask(kind: str, on: str, *words: object) -> object:
  """The way to put a question that is answered now: it makes the act, and gives back what the act came to.
  It is no entry of the bus: it makes the act through act and reads what it came to through peek.
  The call raises the refusal an act came to, and gives back anything else, an exception among it.
  An act that is not done when it is made is no answer now, so the call raises Refused.
  """

def span(lo: int, hi: int) -> Show:
  """span(lo, hi) is the show of the lines lo through hi, where a line under one is counted back from the end, so that span(1, 20) is the first twenty lines and span(-20, -1) is the last twenty.
  A span that holds no line shows none, which HIDDEN is, so what a hidden show shows stands in no turns: a command that takes one tells its header and its binding alone, and a read that takes one tells nothing.
  """

def grep(pattern: str) -> Show:
  """grep(pattern) is the show of the lines that the pattern matches, each with its number."""

def differs(old: list[str]) -> Show:
  """differs(lines) is the show of the lines that differ from the lines it holds, which is what a write shows of what came back."""

HEAD: Final[Show] = span(1, 2000)
"""HEAD is the span of the first 2000 lines, which a read without a show is told as."""
TAIL: Final[Show] = span(-250, -1)
"""TAIL is the span of the last 250 lines, which the stdout of a command without a show is told as."""
HIDDEN: Final[Show] = span(0, 0)
"""HIDDEN is the span of no line, which an act takes to tell nothing of itself but its header and its binding."""

def take(*ids: str, inside: bool = True) -> Filter:
  """take keeps the acts it names and everything they made.
  take is given ids and keeps the acts with those ids.
  take keeps everything that the acts with those ids caused.
  take that is not inside keeps every other act, and drops everything the ones it names made.
  """

def read(path: str, show: Show = HEAD, on: str = "") -> Text:
  """A read: whoever serves the path answers it with the text of it, which the read tells by the lines the model has not seen.
  A read that the World refuses raises Refused in the caller.
  A read on a chain with a source tells the lines of a skipped read again, since they are not known there.
  The engine judges no scheme, so a path of an unknown scheme goes to the World too.
  read is given a path and a show.
  read gives a Text.
  A text without a show is told as HEAD, which is the span of its first 2000 lines.
  A read of the name of a prompt gives the program of that ladder, the words of its rungs in order.
  Whether a name is one of the prompts a chain has heard on itself, which is what its doors serve and no other path, a path of no name being none of them.
  A prompt is the door of the program of its ladder, so a read of its name gives the word of every rung of it in order, the words the gate refused among them, which are none at all for a prompt that ran no word.
  A read of a door that a rung of that ladder says leaves the word of that rung out, since a rung is no part of the program it reads.
  One made from inside an act tells itself, with its path and what it was answered, on the scope of that act; one made from outside an act tells nothing, and neither does one whose show is hidden.
  A read answered with what is no text gives that value, and tells it as python shows it.
  """

def write(text: Text, on: str = "") -> Text:
  """A write: whoever serves the path of the text takes its content.
  write is given a text, and gives the text as it is on disk after the write.
  A write that the World refuses raises Refused in the caller.
  The engine tells of a write of a text only the lines that differ from what the caller asked, and of a write a door answers with a value, that value.
  A write takes no show, since what a write would show the word of the model already said: it tells the lines of what came back that differ from what it asked for, and of those, the lines the model has not seen, so a write that the disk took as it was asked tells nothing at all.
  A door that answers a write with more than it was asked for tells the lines it added and no line the model read before.
  A write to the door of a prompt edits the program of its ladder, so the door and the verb are one act.
  The chain answers a write of the door of one of its prompts with the text it took, and makes its rungs again from it.
  A write of a door that a rung of that ladder says leaves the word of that rung out, and that rung is no rung of the chain after it.
  A new file is a write of a Text made of its path and its content.
  """

def peek(at: str, waiting: object = None) -> object:
  """What the act it is at came to, as the record stands where the call is made, which it gives and never raises.
  The operator reads the results of its own acts in the transcript.
  peek gives the value, or the exception itself.
  peek never raises.
  The done of that act filled its outcome, and the life holds every outcome by name, so a peek reads an act that is over, though its generator is gone, and gives what it was given to wait with for an act that is not over.
  A peek at a chain gives None, since a chain never comes to anything.
  A peek at a name of no question gives None.
  The outcome is no slot of the act, so the plain form of an act holds none of it, and an act made again from the record waits until its done is said again.
  """

def turns(on: str = "") -> list[Turn]:
  """The turns of a chain, folded from what it has heard.
  The turns of what a chain has heard: every fact that carries notes stands as a paragraph of them, and nothing else stands at all.
  The turn a model was answered with closes the turn of the operator and stands as the turn it is, and a text stands by the lines it has not seen, which the one that tells it says the show of.
  turns reads the transcript of the chain and asks nothing, so no act is made and the journal keeps nothing.
  A user turn packs one paragraph for each thing told since the last reply, in order.
  The turns of a chain only grow.
  A turn once phrased is phrased the same on every later reply.
  The turns are folded whole at each reply, and a user turn holds every paragraph told before its reply and since the reply before it, so a paragraph told while a reply is in flight goes to the turn after the answer.
  A reply appends the response as an assistant turn.
  The next reply tells everything that the step told.
  The turns hold every answer of the model as the turn it is.
  The turns of a chain show every act the model made, with its result, and turns is how they are read.
  The turns end with a user turn, empty when nothing was told since the last answer.
  """

def module(on: str = "") -> dict:
  """module gives the globals of a chain: the dict that the last module of its transcript carries, in which every rung of the chain runs.
  A chain that holds no module gives an empty dict.
  """

def program(on: str = "") -> dict[str, str]:
  """program gives the program of a chain: the word of every rung that runs on it since its last module, as python, each under the name of that rung, in order, as the runs of its transcript say."""

def standing() -> Standing:
  """standing gives what the chains stand on: the answer of the last stand that the transcript of the root holds, and an empty standing before the first.
  It reads the transcript of the root as it stands where the call is made, so a grant reads the window of an actor off the standing where the answer of its reply lands, and a later life reads at each place of the record the standing that the record held there.
  """

def stand(on: str = ROOT) -> Standing:
  """stand asks the World what the chains stand on and gives the answer, which boot does on the root at the tip of every life.
  Only a stand on the root changes the standing, since the standing has one home.
  """

def clock(on: str = "") -> float:
  """clock gives one reading of the wall clock of the World.
  clock asks the World for a reading of the wall clock, and the journal keeps what it answered.
  """

def chance(on: str = "") -> float:
  """chance gives a number that is at least zero and under one.
  A number the World draws, at least zero and under one.
  chance is a question the World answers, and the journal keeps what it answered, as it keeps every answer of the World.
  """

def gate(word: str, on: str = "") -> list[str]:
  """Whether the word of a rung may run: the gate reads it after the program of its chain, and it finds nothing when the word may run.
  The word of a rung runs only if the gate accepts the word, or if the chain wrote it.
  The gate checks the word of a rung against the rungs before it in record order.
  A response that is not python is a finding like any other.
  The gate gives no finding when the gate accepts the rung.
  The journal keeps what the gate found, since the gate is of the outside, so a later life reads the same findings and asks the gate nothing again.
  """

def cd(path: str, on: str = "") -> str:
  """A cd: the paths of its chain resolve against its path from then on, and it does nothing else.
  cd completes at once and gives the new working directory.
  The chain answers it with the path it was given, and holds it, so what a chain heard is where its working directory stands.
  It is a question and no fact, since a fact a running word says is heard when the word yields, where a question is answered at once, so the paths of that word resolve against the new directory from then on.
  cd tells the path it was given.
  """

def cwd(on: str = "") -> str:
  """The working directory of a chain is the closest cd back in its transcript.
  The working directory of a chain is the directory of the standing it stands on while no cd stands in its transcript, so a later standing moves no chain that a cd moved.
  The World resolves the path of a read, a write and a command against the working directory of the chain, which it reads.
  cwd gives the working directory that the paths of the chain resolve against.
  cwd reads the transcript of the chain and asks nothing, so no act is made and the journal keeps nothing.
  """

def get(about: str) -> Question:
  """The act again, from its name: whoever holds the name of an act is given the act the life holds under it, whole as it stands.
  get gives an act again from the id of the act.
  get and peek enter nothing in the record.
  get and peek read the record as it stands where the call is made.
  A name of no act of the life gives nothing, since the life holds nothing under it.
  """

def pause(id: str) -> None:
  """A pause: while it stands, nothing it is over hears, and what is said meanwhile waits for the wake.
  A control tells a header of its own name, so a model reads what was done to its work.
  pause is given the id of a pending act or the id of a chain.
  A pause stops no reply in flight: the reply returns.
  A paused chain goes quiet as its in-flight work returns.
  A kind a pause stops: it starts its ear, and while a pause over it stands the ear hears nothing, and at the wake it hears everything that was said meanwhile, in order.
  An act made in that time it hears at once, since an act is put to the ears while it is made and to no ear after the one that takes it, so a paused rung takes the wants of its run.
  A pause stands over what is made after it, until the wake.
  A control is on the scope of what it is over, so it takes no chain of its own.
  """

def wake(id: str) -> None:
  """A wake: it ends the pause over the same act, and what waited is heard.
  Delivery carries on the rungs that await the result, on whatever chain.
  A wake on one act lifts a pause of its chain for that act alone.
  wake is given the id of an act or the id of a chain.
  A wake lifts the pause and delivers every held result.
  A wake gates and runs a held response.
  A wake makes a prompt ask its model with the transcript as it grew.
  A wake makes no reply twice and loses none.
  A wake that this life says, and not one that the journal says again, puts every pending act it is over on to the outside, so the World takes each command, wait, prompt to the operator and reply of them, and a model reads the transcript as it grew.
  """

def cancel(id: str) -> None:
  """A cancel of that prompt reaches the acts that its rungs made on the chain with a source.
  A cancel is over the act it names and everything that act made, and each of them is done with CancelledError.
  cancel is given the id of an act, and says a cancel over it.
  A cancelled act completes with CancelledError.
  A cancelled prompt raises CancelledError to whoever awaits it.
  A cancel touches nothing else on the chain.
  The awaiter of a cancelled command raises CancelledError in its step.
  """

def close(value: object, id: str = "") -> None:
  """An act ended from outside, by its name, with a value: it is done with it, and it ends what it made, since a close is a cancel that carries what the act it names is done with.
  The close of the operator enters the record as a fact of its own.
  A close ends the rung of a prompt at its next await.
  The operator closes a prompt of shape None with None.
  The close of the operator delivers to the act of the rung whenever the close comes.
  The operator closes a pending prompt of any actor.
  A rung closes a pending prompt of any actor.
  close is given the result of a pending act, and the id of that act when it is not the prompt of the running word.
  An exception closes a prompt with that exception.
  The close of the operator stands in the transcript with the name of the operator.
  A prompt completes with the exception that the word of the prompt gave to close.
  close is given the value first, since a word that answers its own prompt names no act at all.
  A value closes an act with that value, and a prompt with a value that has its shape.
  A close that answers a prompt with a value that does not have the shape of the prompt raises Refused in the word that said it, so the prompt asks again.
  A close on an act that is over reaches nothing.
  A close said from a word that names no act is over the prompt that made the rung of the word, and over the rung itself for a word its caller wrote, which answers no prompt.
  A close said from a word that retells reaches nothing and says nothing: it stops the word where it stands, so the rung is done with nothing and answers no prompt.
  A close of the prompt of the running word stops that word where it stands, as a raise does, and nothing after the call runs.
  """

def debug(template: Template) -> None:
  """What a word tells of itself as it runs: each interpolation of a template, with its expression and its value.
  The engine tells what a step debugged.
  A debugged header tells one interpolation of a debug, its expression and its value.
  A raised header and a debugged header stand at the place in the run where they happened.
  The transcript holds between the entries what the run of each word raised and debugged.
  debug is given a python template string.
  debug tells each interpolation of the template, with its expression and its value.
  debug tells nothing but the interpolations.
  debug enters nothing in the record.
  It is no act and it enters no record, and it stands in the turns at the place in the run where it happened.
  Outside an act there is nothing to tell of, so it is refused.
  The engine refuses debug outside an act.
  A tell is on the scope of the act it is of, so debug takes no chain of its own.
  """

def wait(seconds: float = 0.0, on: str = "") -> Act[None]:
  """A wait: the World takes it and says it is done when its seconds have passed, and it is over then.
  A wait stands in no turns, since a wait is no work of a model.
  It tells nothing and answers nothing, since a wait is no work of a model.
  """

def rung(word: str = "", retells: str = "", actor: str = "", on: str = "") -> Act:
  """The run of a word on a chain: a word its caller wrote, which it tells, since nothing else did; or, with no word, a turn of a model, which its chain asks for at the turn it gives it and which the World answers, of which it tells nothing, since that turn stands as the turn it is.
  A rung is an act: the run of one word in the globals of its chain, which the chain has the Kernel run.
  The engine tells what a step raised.
  A step that raised nothing and debugged nothing tells nothing.
  The open of a rung with a word is its header and then that word, as its caller wrote it.
  A rung with no word tells nothing where it is made, since the chain tells it as the last line of the turn it asks for it with.
  A rung with no word and no actor takes the default actor of its chain when it is made, and writes it into its actor word, so its reply and its ledger read the one actor.
  The raised header tells the exception as python shows it, which says its type and its message.
  A rung that retells another rung names its acts under that one, so it makes the same acts and shares them.
  A cancel of a rung is the Kernel's to do, since the Kernel is the one running the word.
  The word of a model is python code and nothing else.
  A step that raised keeps what it bound before the raise.
  A rung may await at its top level.
  A rung answers the prompt of its chain with close, wherever in its word the close is said.
  The Kernel gives nothing for a word that ran to its end, since a word that answers says a close and stops there.
  The Kernel gives what a word raised, and a top-level return is no python, which the gate refuses as it refuses any word that is not python.
  The word of a rung runs to its next await and continues when the close it awaits comes.
  The word of a rung answers its prompt with close, which carries the value the prompt is done with.
  The turns of the chain of another prompt tell the rung of a word its caller wrote.
  A word its caller wrote stands in a user turn as python, under the header of its rung.
  rung is given a word and runs it on a chain in the globals of that chain.
  The gate reads the word its caller wrote like any word.
  A rung with a word completes with what that word raises, and with nothing when the word runs to its end.
  A close ends the rung that runs in a prompt at its next await.
  A chain with a source and its origin share the one act, the record's.
  A chain with a source awaits what its origin started, through the name of the act.
  It says its word may run as soon as it holds one, whichever way that word came, and what the chain makes of the word is the chain's.
  The chain has the Kernel begin it in the module of that chain, and the run carries it forward at the done of every act the word waits for, so that the engine owns the order of it.
  It is done with what the word gave, nothing for a word that ran to its end and the exception for a raise, which it tells with its type and its message, which its close then holds none of.
  Of the acts its word made and read at once it tells nothing, since such an act tells of itself or not at all.
  An answer with no text is a word like any other, so the gate reads it, the run gives no value, and the model is asked again.
  A replay makes a rung of its own retelling each rung of the donor it keeps.
  What a rung that retells asks is named under the one it retells, so the life answers it with what it answered then and the World is asked nothing twice.
  A rung that awaits an act nobody settles waits until the operator cancels it, and holds nothing else of the chain.
  A rung that retells says each question it makes as the rung it retells, so the question it makes at a place is the one that rung made there.
  A rung that retells names the rung the record holds and never another rung that retells it, so a second replay makes the same acts and asks the World nothing twice.
  A rung that retells is done with nothing when its word answers, runs to its end or is cancelled, whatever the Kernel makes of the word, and with what that word raised.
  A rung takes its own act, since the engine is the one that runs it.
  A rung with no word holds the word of its model when the reply for it is done, and it is done with the refusal a reply came to.
  """

@overload
def prompt(shape: None, message: str = "", to: str = "", on: str = "") -> Act[None]:
  """A prompt: it makes the rung of one turn of its model, makes another while the rung it made gives no value, and is done with the value, so a rung whose word is refused and a rung whose word raises are asked again alike.
  The driver gives the name of the prompt, and the prompt is awaited for the shape.
  The engine asks the model again after a refusal.
  The operator prompts a model to make the model work.
  A model prompts the operator to tell the model something or to get a decision.
  A model prompts a model to delegate.
  A prompt to a model is a ladder of rungs in the globals of its chain.
  Nothing but a prompt asks a model.
  The value of a prompt on another chain comes back as the value, and nothing is wired between the chains of one life.
  prompt is given a shape, a message and an actor, on a chain.
  prompt gives the prompt, which is awaited for the shape.
  The response of a prompt has the shape.
  None is a shape of its own.
  Without a message, the actor reads the transcript alone.
  A model answers any shape.
  A model asked with the shape None reads the message, works, and closes with nothing.
  A rung need not wait for a prompt of shape None.
  The actor left unsaid is the default actor of the chain when the prompt is made, which the prompt writes into the actor word of every rung it makes, so every life asks the one actor the record holds.
  A prompt to a model runs in steps until the prompt completes.
  The binding of a prompt gives the shape as python shows the expression, as prompt1: Act[int] = Act('prompt1').
  A rung whose word closes nothing ends its step, and the engine asks the model again.
  The completion of a prompt cancels nothing under the prompt but the words it ran.
  The operator prompts on any chain, by the id of the chain.
  A rung prompts on any chain, by the id of the chain.
  A prompt to the operator completes when the operator closes the prompt.
  The response of a prompt on a chain with a source comes to the act that the caller holds.
  It is the ladder of its rungs, and its name is the door of the program of that ladder, which holds the word of every rung of it for as long as the chain lives.
  A word written to its door is a rung of it, which answers it as the word of its model does.
  A prompt to the operator asks no model: the World takes it and shows it, and it waits to be closed; in a later life, one the record shows open is shown again only at a wake, and one the record shows closed is shown no more.
  A prompt to a model takes its own act, since the engine is the one that asks the model.
  Its close tells what closed it from outside, which the one that closed it says, and nothing of what its rung gave, which the rung has told.
  A paused prompt makes no rung until the wake, and a cancel of it is over its rung too, which ends itself.
  The World closes with a refusal a prompt it cannot put to the operator; which shapes the operator answers is the World's law.
  The shape left unsaid is None, which the acknowledgment uses, and any value responds to it.
  A prompt takes any shape, which a close is read against as python reads an instance: of the shape, or of the origin of a generic one.
  A prompt carries the name of its shape as a word, and takes the name as well as the shape, so the journal makes it again.
  The name of a shape is the word a chain says it by, so a shape that holds a class of the engine or of the chain names it as the chain does, under no module.
  The acknowledgment carries no shape and a message that names the act that is done.
  The turns of the chain hold the result of the command that the acknowledgment names.
  A cancelled result is no orphan.
  The response of an acknowledgment is no orphan.
  When an act that a rung of the chain made is done while the word of that rung does not run, no reply has shown it, no prompt it heard on itself is open and no word of the chain is running, the chain prompts nothing, so that the model sees it.
  A pause stands over the close that answers a prompt too, so what a word gave waits for the wake.
  A prompt tells its message and its binding where it is made, as every act tells its open, whether a model or the operator answers it.
  """

@overload
def prompt[T](shape: type[T], message: str = "", to: str = "", on: str = "") -> Act[T]: ...
@overload
def prompt(shape: str, message: str = "", to: str = "", on: str = "") -> Act: ...
@overload
def prompt(shape: object, message: str = "", to: str = "", on: str = "") -> Act: ...
def chain(label: str = "", source: str = "", filter: Filter | None = None, on: str = "") -> Act[Never]:
  """chain says what a chain does: how it is opened, what it tells, and what it answers for.
  boot gives the root, and chain gives the chain, which never settles.
  The header of a chain with a source carries its label and its source.
  The entry that opens a chain with a source carries its label, after the prefix.
  chain gives the new chain, which never completes.
  boot gives the root as an act of Never, and the root never completes.
  The engine makes a chain in one way: by running its rungs.
  In a chain with a source the rungs of the origin up to that source run again in the module of the new chain.
  A chain given a source stands on that one: it retells the words of it as they stand, each rung of it retelling a rung of that one, so that it makes the same acts and shares them, and what it holds of the transcript of that one is what its filter kept, though it runs every word all the same, so what it holds bound is more than its turns say.
  A prompt to an actor its roster does not hold it closes with the refusal.
  A chain that the word of a rung opens is a scope of its own: its words are on itself, though the chain fact itself stands on the chain of the rung that opened it; and when that rung is retold, the word makes the same chain, since a rung that retells another shares the acts it makes, so a word that opens a chain opens it once.
  The engine refuses a prompt to an actor outside the roster.
  The chain holds the control it says itself, so what it closes tells the model what was done to it.
  The engine binds the exception of a raise in the globals of the chain, under the name raised.
  The engine binds the exception again at each raise.
  A chain: its module, which is the engine itself, named for the chain, since everything the file defines is the model's to call and nothing of it is bound to one chain, and what the operator would add to it, it makes a rung of, which binds it, stands in the program and is said again in a later life, a fact said from a run being on the chain of that run, and a word that wants a chain of its own giving the name of its own as the source.
  A chain is chainN whether boot or chain opened it, and the root is chain1.
  A chain is an act of Never, so the chain never completes.
  The turns of a chain tell the standing and the acts of the operator.
  The transcript of a chain is the facts on it, in the order they were said.
  Nothing leaves a transcript once the transcript holds it.
  An assistant turn keeps the role assistant in every chain made from the chain.
  The engine adds the acts that caused a kept act to what the filter kept.
  The globals of a chain whose filter is a take that is not inside hold the bindings of the skipped words still.
  A chain with a source made after the rung whose word defined a door has the door too.
  The globals of a chain are those of a module named by the id of the chain.
  Every name that the file defines is in the globals of a chain.
  A chain holds whole every act made on the chain, the rungs it makes itself among them, which tell nothing.
  A chain holds every fact on it, and an act on another chain is that chain's.
  A prompt that a step of another chain made reads nothing of that chain.
  The word of a rung rebinds the default actor like any name, and so does an answer of a stand that changes the standing, and the last binding in record order wins.
  The transcript of the root begins with the open of the root and then the standing.
  The transcript then holds the prompt of the operator, and the replies of the model, each with its turn.
  The engine reads its own names through the globals of the chain.
  The engine uses a rebound name from the next use on.
  No close reaches a chain, since a chain never completes.
  To await a chain never returns.
  A chain has a globals dict and a working directory of its own.
  A chain with a source holds the acts it inherited from that source, as the filter kept them.
  The steps that an inherited prompt takes after the point enter the transcript of its owner alone.
  What a chain binds is its own, and a chain with a source is how a chain gets isolation.
  Two chains from one source hold the same values, since both ran the same rungs.
  A chain with a source inherits the default actor with the globals of its origin at that source.
  A chain with a source that holds an act reads the close of the act in its transcript.
  A chain with a source awaits or peeks an inherited act as it likes.
  The engine does not wake a chain with a source for the result of an inherited act.
  The globals of a chain with a source may hold more than its turns say.
  The transcript of a chain with a source holds the entries up to that source first, then the entry that opened it.
  The acts of the prefix of a chain with a source keep the ids they had on the origin.
  The globals of a chain with a source are those of a module of its own.
  The objects of a rung of a chain with a source are that chain's own, made again, and only code is shared.
  The globals of a chain with a source are the origin's at that source, whatever the filter kept.
  A chain with a source inherits the words its caller wrote with its prefix, wherever its source.
  A filter that skips a rung its caller wrote keeps it out of the turns of the new chain.
  A chain with a source holds the classes that its origin defined before that source.
  A chain with a source reads nothing that its origin did after that source.
  What a rung of a chain with a source binds lands on that chain and not on its origin.
  The origin holds the chain fact of a chain that a rung of it made, and nothing of that chain's own.
  Two names of the module are the chain's to bind: the actor it stands on, and what the last rung that raised raised, so that the word of a rung reads what the word before it came to.
  It holds the transcript of its origin first and tells its own open after it, and the rungs it runs again tell nothing, since every word of those stands in what it inherited.
  It hears no control that ends it, since nothing that happens to a chain ends it, and a control over a chain is over the acts on it, which end themselves.
  Every done it holds answers a question it holds, since it holds every question made on it, the ones it made itself among them.
  What a chain with a source holds of the transcript of its origin: what its filter kept, everything that made what it kept, the reply of every rung it kept, which cuts its turns where the origin asked its model, and the open of the origin, which tells the standing.
  The filter is given every act, what the operator asked among them, since what the operator asked of a chain no rung of it says again.
  What a chain said of itself it keeps whatever the filter says, so the open of the origin stands in the new chain.
  The source of a chain is a chain, by its name, and means the transcript of that chain as it stands.
  A source that names no chain of the life refuses the call in the caller, and no chain is made.
  A chain with a source reads what its origin stands on, as it stands.
  The chain retells the rungs of its origin through the program of the origin, and it makes the rungs it retells itself, so they tell nothing.
  A replay makes the rungs of a chain again from its donor: it keeps each rung of the ladder while the words it is given repeat it, it makes one rung of what is left, and every rung of the chain after the first word that differs is gone.
  A replay makes the module of the chain again, as it was at its birth but on the standing the chain stands on then, and makes its rungs in that one, so what a word it drops bound is gone, and a word that runs while it happens ends in the module it began in.
  The donor of a replay is the rungs of the origin for a chain with a source, and the rungs of the chain as they stand for a write of the door of one of its prompts.
  A write of a door gives the words of that ladder alone, so a rung of the chain that is no rung of that ladder and stands before the first word that differs stands as it did.
  Before every reply the chain tells the last line of the turn, which says what the answer is for, as #rung5 advance on prompt1, which names the one that made the rung.
  Where it asks, the chain makes a rung of the statements the turn shows of every act but a rung, which bind the name of each act the turn opened; the gate does not read that rung, since the engine wrote it, and the turns do not show it, since the turn shows its statements already.
  The chain takes its own act, since the engine is the one that runs it.
  The chain asks the model of a rung for its word by a reply it makes under the site of that rung, whose one word is the actor.
  """

def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act[None]:
  """A ceiling on a chain, in dollars, in the share of the window that one answer fills, or both: it holds the ledger of the chain from the moment it is made, the dollars of the answers since then and the share of the window the last one filled, and it tells that ledger at each answer of a model, so no turn a reply has sent grows a line after it.
  grant on a chain puts a ceiling on it: dollars, a share of the window, or both.
  The engine enters a pause on the chain when a response carries the ledger to its ceiling.
  The word of the response that crossed the ceiling runs.
  No reply follows the response that carried the ledger to the ceiling, until a wake.
  The model continues after a later grant and a wake.
  An answer that carries the ledger past the ceiling pauses the chain, so the word that answer brought runs and what it gave waits, and no rung of the chain asks until the wake.
  Lifting a ceiling wakes nothing: the pause stands until a wake, ceiling or no ceiling.
  It stands until it is lifted, as the chain it is on does, so what it comes to is what lifted it: nothing for a later grant that closes it, and a CancelledError for a cancel.
  A grant of nothing, of a ceiling under zero, or of a share past one is no ceiling: it is done with the refusal, which whoever made it takes by awaiting it, and it tells nothing, since it never stood; a prompt to no actor of the roster is closed the same way after it has told its open.
  A later grant that stands closes every grant of the chain before it that stands, and none that is over, so the ledger counts from the new one alone; one that is no ceiling closes nothing, and the ceiling that stands stands on.
  A cancel of it lifts the ceiling, since it is an act like any other.
  A grant is any caller's, on any chain.
  A grant finds the grants of its chain among the acts of the life, so a grant on a chain with a source closes no grant of its origin.
  """

def bash(
  command: str,
  fed: bool = False,
  timeout: float = TIMEOUT,
  show: Show = TAIL,
  show_err: Show | None = None,
  on: str = "",
) -> Act[Exit]:
  """A command: its streams as they come, its exit, the door of its streams and of its stdin, and what it came to.
  bash is given one show for each stream that bash tells.
  A feed whose text is None closes the stdin of the command.
  The commands of the World run at the same time.
  bash is given a command, a fed flag, a timeout, and a show for each stream, the plain words first, so the journal makes it again.
  bash gives the command, which is awaited for its exit code and its streams.
  The World runs the command in the working directory of the chain, which it reads.
  The command runs until it ends, until its timeout, or until a cancel.
  The World ends the command at its timeout.
  The command completes with its exit code and its streams.
  The exit code of the command is None after a timeout.
  The stdin of the command is closed unless bash opened the command fed.
  Without a stderr show, the stderr of the command flows into its stdout.
  The stderr door of a merged command stays empty.
  A command without a timeout has the timeout TIMEOUT that the file names.
  The World that takes a command answers it: it says it done with its Exit, the code and the streams it kept, since the owner of an act is the one that answers it.
  The doors bash1/stdout and bash1/stderr of a command bash1 are readable while the command runs and after it.
  The door bash1/stdin is writable while a fed command runs.
  A write of nothing to bash1/stdin closes the stdin.
  A read of bash1/stdout gives the lines of the command while the command runs.
  A wake on a chain with a source starts no inherited command again, since only the owner starts an act.
  The World takes it, and a later life has the World take it again only when the record shows it started and not ended, and then only at a wake.
  Its stdin is written while it runs and it is fed, which it says to the World as a feed and answers with the text that landed, and a write of nothing closes it; a write of it takes no word once the command ended, and none at all when the command was not opened fed.
  Without a show of its own, the stderr of it flows into its stdout, and the door of its stderr stays empty; with a hidden show it tells its header and its binding alone, and not its command and not its close.
  It answers a read of a stream while it runs, and once it has ended it lives on to answer a read of its streams and to refuse a write of its stdin, and nothing else reaches it, so a cancel does not end it, as it ends every other act.
  It runs until it ends, until its timeout or until a cancel: the World is the one that ends it, at the timeout it reads off the act as at a cancel, since the World is the one running it, and the engine says nothing to make it.
  A pause stops no command: it runs on, the World says it done when it ends, and its ear tells its exit at the wake.
  Its close tells what its stdout shows, and what its stderr shows when that stream has a show of its own that is no hidden one, so a merged command tells no stderr.
  The stdout of a command without a show is told as TAIL, which is the span of its last 250 lines.
  The engine refuses a write to bash1/stdin once the command ended or was cancelled.
  The World asks a command whether its stderr flows into its stdout, and the command answers from what its verb was given.
  The World hears the bash itself, with the command, the fed flag and the timeout, and no working directory and no show.
  """

@dataclass
class Text:
  """A text: its path, what stands at it, of which its lines are the lines, and the text it came from.
  The lines of a text derive from its content.
  Every edit of it gives another text, which came from this one.
  """

  path: str
  content: str = ""
  before: Text | None = None
  @property
  def lines(self) -> list[str]: ...
  def grow(self, text: str) -> Text:
    """The text as more of it is told, which is how a stream of a command grows, and which comes from no text, since a stream that grows is no edit of one."""
  def edit(self, lo: int, hi: int, lines: list[str]) -> Text:
    """The text with one more edit; an edit whose lines are not there is refused.
    The edits that say themselves are one edit each: a replace of a string, an append at the end, an insert before a line, and a delete of lines.
    """
  def replace(self, old: str, new: str, once: bool = False) -> Text:
    """replace(old, new, once) gives a new text with the edit added."""
  def undo(self, n: int = 1) -> Text:
    """undo(n) gives a new text without its last n edits.
    The text before its last n edits, and the text itself when it came from none.
    """
  def append(self, text: str) -> Text:
    """append(text) gives a new text with the text added at its end."""
  def insert(self, line: int, text: str) -> Text:
    """insert(line, text) gives a new text with the text put at that line."""
  def delete(self, lo: int, hi: int) -> Text:
    """delete(lo, hi) gives a new text without the lines lo through hi."""
  def find(self, pattern: str) -> list[int]:
    """find(pattern) gives the numbers of the lines that the pattern matches.
    The numbers of the lines the pattern matches, which is what grep picks of them.
    """

@dataclass
class Exit:
  """What a command came to: its code, and each of its streams as a text, in the order the command keeps them.
  An Exit is the value that a command completes with.
  The streams of an Exit are its stdout and its stderr, each a Text.
  """

  code: int | None
  stdout: Text
  stderr: Text

class Act[T = object](str):
  """The name of an act, which is what a verb gives and what a caller holds of the act: a string, so it names the act to close, cancel, pause, peek and get, and awaitable, so it gives what the act comes to.
  An act is over when its done stands and lives until then; there is no other state, and a control over an act that is over reaches nothing, but a wake of the ear of a command that a pause holds.
  The outcome of a cancelled act is the CancelledError it completed with.
  Only a command lives past its done, and every other act is dropped at its done.
  """

  def __await__(self) -> Generator[Self | Future[T], object, T]:
    """To await an act gives the value of the act when the act completes.
    An act is awaited from any chain.
    A rung that awaits an act reads the result of the act.
    A rung awaits an act and nothing else.
    To await an act raises the exception that the act completed with.
    To await a cancelled act raises CancelledError.
    A word that awaits a chain raises Refused where it waited, since a chain never settles and the word could go no further.
    The awaiter of the prompt raises that exception.
    A run that awaits it hands it to whoever steps the run, since the engine owns the order of every run; the operator, which the engine does not step, waits on its own loop.
    """

class Refused(Exception):
  """What a call the engine will not make raises in the one that made it.
  A refused call raises Refused in the caller.
  """

class Drift(Exception):
  """What a life fails with when an act of it is not the one the record holds, which the journal raises, so that it comes out of the entry the operator went in by and the life goes on with nothing.
  An act whose words are not the ones the record holds under its name is a drift, which it raises.
  A drift breaks the journal, which keeps nothing more, and the life runs on with nothing kept.
  """

type Note = str | Showing
"""One thing a tell says: python as it stands, or a text and its show, which the fold shows as comments by the lines the model has not seen.
A paragraph is what one fact that tells stands as in a turn: its notes, one after the other, and a blank line between two paragraphs.
The first line of a paragraph is its header: # and, with no space, the id of the act it is of, or the kind of the read, the write or the cd it tells, then its words, as #bash1 exited 0 or #read a.txt.
A paragraph may hold more headers of what it is of, each on a line of its own right under the first, and every other comment of it begins with # and a space, so no line of a message or of a text reads as a header.
The header of a paragraph names the act it is of by its id, what the act tells and a control over it alike, and the paragraph of a read, a write or a cd stands at the place in the run where it was asked.
The headers of the file are the open of an act, closed, exited, raised, debugged, refused, ledger, roster, cwd, actor, advance, paused, woke, cancelled, and one for each question that tells: read, write and cd.
A statement that a paragraph shows binds the name of an act in the chain, and a comment binds nothing.
"""
type Showing = tuple[Text, Show]
"""A text a note shows, and the show of it, which is what a tell of a text carries and what the fold of the turns makes comments of.
The paragraph of an exited command holds one showing for each text told.
A tell shows each showing it holds, and each other note stands as it is.
A showing stands as a comment of the path of its text and of how many of the lines the show picked the chain knows, then a comment for each other line it picked, with its number.
The engine applies a show before it writes a line, so the paragraph holds the picked lines alone.
A show applies to a text or to a stream.
A line told once on a chain is known there, by its path, its number and its content.
read tells a line again after the content of the line changed.
A text costs its size once on a chain.
A second read of a text tells the model no line that an earlier read of the chain told.
"""
type Usage = tuple[int, int, int, int, float]
"""What one answer of a model cost: the words it read and wrote, of which the words it read again and the words it kept to read again, and its dollars.
A usage holds the token counts and the dollars of one model response.
The share of the window it filled is the words it read against the window of the actor its rung names, in the standing where the answer lands, so no word of it says the share.
"""
type Turn = tuple[Literal["user", "assistant"], str, Usage | None, object]
"""One turn of what a model reads: who said it, the python it holds, what the answer to it cost, and the blocks of the provider, which are its own, are read by nothing of the engine, and go back to it with the turn.
The role of a turn is assistant for a response, and user for everything else.
The blocks of an assistant turn are the response as the provider returned it.
A turn is a role, python, a usage and blocks.
An assistant turn carries the usage of the response.
An assistant turn keeps the role assistant for every model that reads the turns.
The engine sends the response to the provider again whole.
The python of an assistant turn is the word the model wrote, quotes and all, and the python of a user turn is the paragraphs told since the reply before it, and nothing else.
"""
type Actor = list[str | list[str] | int]
"""An actor the World offers: the name of a model, the efforts it takes, and the window it reads.
The window that a roster entry leaves unsaid is the window that the file names.
What a prompt names is one of these names and one effort of that range.
An actor takes an effort of its own, and any actor takes the effort that is not named.
"""
type Standing = list[list[Actor] | str]
"""What a chain stands on: the actors the World offers, the directory the chain starts in, and the actor a prompt goes to when it names none.
The roster, the directory and the actor that a model reads are in the transcript of its chain.
A standing holds no source: the engine is one file the model imports, and a record made by another engine is a drift.
"""
type Entry = tuple[Fact]
"""One entry of the record: the fact, an act among them, and the answer of an act after it, as every other fact about it.
The order of the record is what puts an entry back in its place in a later life.
The World keeps each entry as the journal says it, plain or not.
The record is a sequence of entries about acts.
"""

type Done = tuple[Literal["done"], str, str, object]
"""What an act came to: a done settles the act it names.
A result enters the transcript whether or not anyone awaits it.
An act that is over says nothing, and a command lives on to answer its doors.
A done that an act said itself is the result of the act.
A done that an ear says while the act is put to it is the answer to the act, which takes it now.
A kind that ends when it is told to: it starts its ear, and then a done that names it is what it came to; a cancel over it ends it with a CancelledError, and a close of it with the value that close carries.
"""
type Tell = tuple[Literal["tell"], str, str, list[Note]]
"""A tell carries notes about the act it is about, and the turns are folded from them.
What an act tells is the telling of its chain, so the journal keeps none of it, and a later life tells it again.
A fact that carries notes is what the turns are folded from: a tell, and a control, which carries the header it tells.
"""
type Pause = tuple[Literal["pause"], str, str, list[Note]]
"""While a pause stands, nothing that the pause is over hears, and what is said meanwhile waits for the wake.
A control is a fact over an act: over that act, over everything that act made, and over everything on a chain when the control names a chain.
A control reaches what it is over, and whatever else the words of the control name.
A control is no act: it takes no name of its own, and the record holds it as a fact about the acts it is over.
What a control reaches: the act it names, everything that act made, and every act of the chain it names.
It reaches by the chain as well as by the name, since an act on a chain is not under it unless the chain made it.
A control carries the header it tells, so a model reads what was done to its work whoever did it, and nothing else writes that header: the chain that pauses a chain at its ceiling, or closes a prompt it will not serve, says the control the one way there is to say it.
A pause is over the act it names and everything under it.
"""
type Wake = tuple[Literal["wake"], str, str, list[Note]]
"""A wake ends the pause over the same act, and what waited is heard.
A wake is over the act it names and everything under it.
"""
type Cancel = tuple[Literal["cancel"], str, str, list[Note]]
"""A cancel ends everything it is over: each of them is done with CancelledError, and none of them says anything of its own again.
A cancel reaches to any depth, and on whatever chain.
"""
type Close = tuple[Literal["close"], str, str, object, list[Note]]
"""A close is a cancel that carries what the act it names is done with, and it is a kind of its own, since a tuple has no slot that may be empty.
A close is over the act it names, the words running under it and the replies that ask for those words, where a cancel is over everything under it.
"""
type Started = tuple[Literal["started"], str, str]
"""What an ear says to take an act whose done comes later, and which no ear after it hears.
The started names the act and says no more of it, since the ear heard the act itself and nothing is told twice.
The journal keeps a started of the outside, so a later life holds the act from the outside and says no started for it, since only the outside runs it: the act is done where the record holds its done, and pending when the record holds none.
"""
type Keep = tuple[Literal["keep"], str, str, Entry]
"""One entry of the record, kept and said: the journal holds it for this life and the World for the next.
The journal says one keep per entry, and the World keeps it as plain data if it likes.
"""
type Reply = tuple[Literal["reply"], str, str, str, str]
"""The request of a reply is the transcript of the chain as turns, which the World reads when it takes it.
The model reads the turns of the chain at each step.
A reply carries the rung it asks for as its maker, the chain that asks as the chain it is on, and the actor as its one word, and nothing else.
A reply is on the chain that asks, so the World keys its facts and its cache by chain.
A reply the World cannot answer is the World's to refuse: it pauses the chain first when it wants a wake, and is done with the refusal, which the rung comes to and the prompt asks again after.
The life refuses a reply that no ear owns, and the chain that asks then pauses itself, since that fault stands until a wake.
A reply ends as a wait does, so a cancel or a close over it ends it with a CancelledError.
The World reads the turns of the chain whole when it takes a reply, folded again for that reply, and the journal keeps the answer and no turns.
The World answers a reply with a done whose value is the turn of the model, which stands as the turn it is, with its usage and the blocks of the provider.
Many prompts are pending on one chain at once.
A chain has at most one reply in flight.
Across chains there is no limit on the replies in flight.
Every model asked on a chain reads all the turns of the chain, as the turns grow.
As many replies as there are chains are in flight together.
A later life asks no model, and does no act, whose close the record already holds.
A new prompt reads the whole transcript of the chain, the cancelled work included.
No prompt that a pause is over asks a model.
A paused chain makes no new reply after a held response.
No model is asked for a rung the journal answered, since a later life asks again for nothing it was answered once.
It asks for no rung a pause stands over, whether the pause is over that rung or over the chain, so a paused chain asks no model until the wake.
The chain asks one model at a time, which it reads from its transcript, the rung that has waited the longest among those it heard on itself that nothing has been said of, handing it the turns as they stand, for the World to hand its provider as it likes, so that many chains ask many models at once.
"""
type Ready = tuple[Literal["ready"], str, str, str]
"""A ready says the word a rung holds.
The word of a rung enters the program of the chain when the gate accepts it or when the chain wrote it, and runs in the globals of the chain.
The word of a rung that extends the engine is part of the program, so the extension returns in a later life.
The old words stay in the program and in the turns after a rung rebinds a name.
"""
type Run = tuple[Literal["run"], str, str, str, str, str, str]
"""A run is the act of running the word of a rung, which the chain makes on itself and the Kernel takes: it says started as the run, runs the word as the rung, and says the run done with what the word gave.
The word a run carries is python, which unquoted made of the word of the rung.
A run names the rung that the word retells, so a Kernel may answer a retold run from what it kept of that one instead of running the word again.
Every rung of a chain runs in the globals of the chain, the word of a model and a word its caller wrote alike.
What a rung binds stays bound for every later rung of the chain.
The last rung to bind a name wins.
One rung runs at a time on a chain, and rungs interleave at their awaits, whether they are rungs of one chain or of many.
The word of a rung runs again in every chain made from its chain and in every later life, and what it does outside its acts it does again.
Two prompts on one chain see the bindings of each other as they run.
A rung whose word rebinds a broken name repairs the chain, since the last rung to bind wins.
A prompt after such a rung finds what it bound.
The chain runs one word at a time.
A word that waits for an act gives the chain to the next word, which runs while it waits, and the waiting word runs on at the done of what it awaits.
"""
type Wants = tuple[Literal["wants"], str, str, str, str]
"""A wants is the act that a run makes when its word waits for an act that is not done: the rung takes it, and answers it with what that act came to when the rung hears its done, so a pause over the rung holds the word."""
type Out = tuple[Literal["out"], str, str, str, str]
"""The streams of a command come as out facts while the command runs, which the journal keeps.
A later life reads the parts that a command told before the death of the process.
"""
type Feed = tuple[Literal["feed"], str, str, str | None]
"""A write to the stdin door of a fed command hands the World a feed with the text."""
type Read = tuple[Literal["read"], str, str, str, str]
"""A read is the question of the text at a path."""
type Write = tuple[Literal["write"], str, str, str, Text]
"""A write is the question of putting a text at its path."""
type Clock = tuple[Literal["clock"], str, str, str]
"""A clock is the question of a reading of the wall clock."""
type Chance = tuple[Literal["chance"], str, str, str]
"""A chance is the question of a number the World draws."""
type Stand = tuple[Literal["stand"], str, str, str]
"""A stand is the question of what the chains stand on, which boot asks the World on the root at the tip of every life, and whose answer binds the actor and the directory of every chain from its place on.
A change of the World between two lives enters the transcript of a chain.
The World answers a stand with the roster, the directory and the actor.
A model asked on any chain of a later life finds the new roster in the transcript of its chain.
The World answers it with a done at once, so the life stands on that answer before boot returns.
The journal keeps each stand and its answer, so a later life says them again at their places and replays every chain on what it stood on there.
At its tip, once the record is said again whole, boot stands the life again, so a change of the World reaches every chain after what it replayed.
Every chain hears the done of every stand, and a chain whose standing that answer changes binds its default actor and tells it there, so the transcript grows at one end.
Each standing binds the default actor of the chain, under the name actor.
The chain tells each standing it takes in one paragraph of three headers, one for each part: the roster under the header roster as python shows it, then the directory under the header cwd and the actor under the header actor, each as it is.
The standing a chain tells is what its transcript holds of it, and the stand itself tells nothing.
"""
type Module = tuple[Literal["module"], str, str, dict[str, object]]
"""A module carries the globals of a chain, which the chain says at its birth and at each replay, so its transcript says which module each of its rungs ran in."""
type Prefix = tuple[Literal["prefix"], str, str, list[Fact]]
"""A prefix carries what a chain with a source holds of the transcript of its origin, which the chain says at its birth, before its open.
The transcript of the chain holds the facts of its prefix where the prefix stands, and not the prefix itself.
"""
type Gate = tuple[Literal["gate"], str, str, str, str]
"""A gate is the question of whether a word may run, which the ear named gate answers with its findings, apart from the Kernel, so a word may ask it while the Kernel runs that word.
A gate carries the word alone, and the gate reads the program of the chain before that rung when it takes the gate, and the word after it, so the Kernel keeps no ladder of its own, the journal keeps no program, and a word of a program made again is read after the rungs that stand.
The refused paragraph holds the findings that refused the word of a rung, one comment for each.
The chain has the word of a rung gated before it runs, but a word it wrote itself, and a refused word runs never.
A rung that retells stands with the gate where the one it retells stood, so the gate reads a word once in a life, and a copy of a refused word is refused again and tells its findings not again.
A refused word of a rung stands in the ladder of its prompt, which its door shows, and it is no part of the program of the chain, which holds the words that run.
The chain tells the findings that refused a word, ends that rung with a refusal that holds none of them, and the prompt of it asks again as it does for a word that gave no value.
"""
type Cd = tuple[Literal["cd"], str, str, str, str]
"""A cd is a question that answers with its path, which the chain holds."""
type Merged = tuple[Literal["merged"], str, str, str, str]
"""A merged is the question of whether the stderr of a command flows into its stdout, which the command answers from what its verb was given, and which the World asks when it takes the command."""
type Wait = tuple[Literal["wait"], str, str, str, float]
"""A wait carries the seconds that must pass before the World is done with it."""
type Rung = tuple[Literal["rung"], str, str, str, str, str, str]
"""A rung carries the word, the rung it retells and the actor."""
type Prompt = tuple[Literal["prompt"], str, str, str, str, str, str]
"""A prompt carries the name of the shape, the message and the actor of a prompt."""
type Chain = tuple[Literal["chain"], str, str, str, str, str]
"""A chain carries the label and the source of a chain, and every chain is chainN whether boot or chain opened it."""
type Grant = tuple[Literal["grant"], str, str, str, float | None, float | None]
"""A grant carries the ceiling in dollars and the ceiling in share of the window."""
type Bash = tuple[Literal["bash"], str, str, str, str, bool, float]
"""A bash carries the command, the fed flag and the timeout, and no show and no working directory."""

def under(name: str, of: str) -> bool:
  """Whether one act is another or was made by it, which the life says, since every question says who made it.
  An act is under every ancestor of the act, which the maker of each says in turn, up to the operator or an ear of the outside.
  An act is under its chain only when the chain made it: the rung of a prompt the operator made is under that prompt, and on the chain, so the maker of an act says who made it and never where it stands.
  Nothing is under a name of nothing.
  """

def acting() -> str:
  """The run a fact speaks from, which is the name the site holds when that name is an act's, and nothing at all for the operator and for the World, since a fact of theirs is said from no run."""

def question(a: Saying) -> bool:
  """Whether a fact is a question, which its name says: a question is about itself, and its name is under its kind."""

def scope(name: str) -> str:
  """The scope of a question, from its name: the chain it is on, and itself for a chain, and nothing for a name of no question of the life."""

def tell(name: str, text: object = "", *notes: Note) -> None:
  """What a question answered now that shows a text or changes a state tells of itself: a paragraph headed with its kind, its words and what it was answered, said on the run that asked it, and nothing at all outside a run; one that only reads a value tells nothing, since the word that asked it holds the value, which it debugs to see.
  A question is put to the ears of the engine before those of the outside, so the World is asked for nothing that the engine knows.
  """

def tells(id: str) -> bool:
  """Whether an act tells: a rung that its chain made tells nothing, neither its word nor what its word does, since what it would tell stands told already."""

def told(id: str, text: object = "", *notes: Note) -> Tell:
  """The open of an act tells the id and what the act says of itself, and no actor and no arguments as such.
  A closed header tells the act with what it came to, as python shows it.
  told gives the saying of a tell about an act, with one paragraph headed with the id of the act, which an ear yields and a verb says, so a chain holds what it told where it told it.
  """

def control(kind: str, name: str, id: str, *words: object) -> Fact:
  """A control said over the act it names, with a header of its name headed with that act, which is how pause, wake, cancel and close say theirs.
  The call gives the control as the life made it whole, so a close that named no act reads which act it is over.
  A control is said while the act it names is not done, and a wake while it is paused too, so a control that reaches nothing says nothing.
  """

def headed(name: str, text: object = "") -> str:
  """The header of a paragraph: # and the name with no space between, then the text, whose later lines are comments."""

def commented(text: object) -> str:
  """The text as comments: # and a space before each line of it, and # alone for an empty line, so no line of it runs."""

def bound(id: str, of: str = "object") -> str:
  """The statement that binds the name of an act to the act, with the type of what the act comes to, as bash1: Act[Exit] = Act('bash1'), so the gate knows what an await of it gives."""

def showing(got: object, show: Show) -> list[Note]:
  """What a paragraph shows of what a door answered: the text by the lines the model has not seen, and anything that is no text as a comment of how python shows it."""

def shown(pair: Note, seen: dict[str, dict[int, str]]) -> str:
  """The lines of the text the model has not seen, and how many of the rest it knows."""

def unquoted(word: str) -> str:
  """What a word is as python: each quote in it bound as a string.
  A quote is a string a word writes between two marks, <Sn> at the start of a line and </Sn> at the end of a line, n being any number, so nothing in it needs an escape.
  The open mark is looked for line by line from the top, and its close mark from the last line up, so a quote may hold the marks of another.
  The value of a quote is the text between its marks, less a line break just after the open mark.
  A quote becomes Sn bound to its value as python writes it, on the line of the open mark, and each other line of the quote becomes an empty line, so every line after it keeps its number.
  An open mark that has no close mark stays as it is, and the gate reads it as the python it is not.
  The engine unquotes a word before the gate reads it and before the Kernel runs it, and the door of a ladder and the turns keep the quotes as the word wrote them.
  """

def offered(roster: list[Actor], to: str) -> int | None:
  """The window an actor reads, and nothing at all when the roster holds no such actor, or when that one takes no such effort."""

def covers(a: Fact, id: str) -> bool:
  """Whether a control is over an act: over the act it names and everything under it, and over every act on the chain it names; a close is over the act it names, the words running under it and the replies that ask for those words, where a cancel is over everything under it."""

def paused(id: str) -> bool:
  """paused reads whether an act is paused off the transcript of its chain, so an ear born while a pause over its act stands is born paused, and takes its act all the same."""

def ended(a: Fact, id: str) -> object:
  """What an act a control is over is done with: the value a close carries for the act it names, and a CancelledError for every other."""

def idle(id: str) -> Generator[None, Fact]:
  """idle hears every fact and says nothing of its own, which is the ear of a wait and the ear of the operator."""

def lives(g: Ear, a: Fact | None) -> bool:
  """lives carries a fact into an ear, says everything the ear yields, and gives whether the ear lives on."""

def pausing(ear: Callable[[str], Ear]) -> Callable[[str], Ear]:
  """An act is paused while the last control in record order that is over it is a pause.
  A pause holds delivery: a result that arrives enters the record and waits.
  A paused prompt stops at its next boundary, with its loop where it stood.
  The engine holds the response of a reply that returns on a paused chain.
  A rung carries on only while its own chain is not paused.
  A control from outside reaches a paused act at once, where what the words of the act say waits for the wake.
  A paused ear hears at once an act put to it, and every other fact at the wake.
  """

def ending(ear: Callable[[str], Ear]) -> Callable[[str], Ear]:
  """A close from outside still ends what a pause is over.
  It is over as it says a done of its own, so it never hears that done and says nothing after it.
  """

def boot(record: Sequence[Entry] = (), **outside: Ear) -> Act[Never]:
  """A life: everything that is said in it is said here, so the log of what was said, the generators that listen by their names and the tables of the life are its own, and it binds the names that reach them: say, which says a fact, act, which makes an act, drive, which brings a generator to life, and get, peek and transcript, which read the tables.
  A verb from a rung takes the chain of the rung when the call leaves on unsaid.
  The kind of an act is the verb that made the act, or the kind an ear made it with.
  An act says who made it, and the one that made it says its own maker in turn, which is how the life knows every ancestor of an act.
  An act that the operator made has the operator for its maker.
  A name is never reused in the record.
  A later life gives the same names, since the same acts make them again.
  The root has no parent.
  A question goes to the living acts of the engine before it goes to the World, since the engine settles what it knows before the outside reads it, and asks the outside for nothing that the engine can answer itself.
  A later life makes a door again from the word of the rung that defined it.
  The record that boot is given enters nothing in the record, since the record is what boot is given.
  An act takes its name when the act opens, and the name says what made the act.
  The engine derives the transcripts, the turns, the globals and the working directories from the record.
  A fact the journal says again is the journal's own, as is an answer it says again when an act is made again.
  What the module holds is not in the record.
  The root is the first act of the record.
  A later life on a kept record makes the root again and enters no second root.
  The engine appends after the last entry of the record boot was given.
  What the word of a rung made or computed, a later life makes again by running the word.
  The engine makes a chain from the record and in no other way, by running its rungs again.
  The rungs of a chain run in record order.
  Each act a rung makes again is the act the record holds at that place, with its result.
  A replay makes the same acts in the same order and gives them the same ids.
  The journal answers what the record holds an answer for, the gate among them, so the outside is asked nothing it answered once.
  A later life on a kept record starts nothing and keeps the ids of the earlier life.
  In a later life the rungs of every chain run again from the record that the World kept.
  The engine serves the doors of the file itself, and asks the World for nothing.
  A later life reads the same text from a door.
  It is given what the World kept of the life before it, and the generators of the outside, the Kernel, the gate and the World among them, each under the name it is to hear by, and it brings them to life with its own.
  It opens the root, the first act of any record, which every life opens under the one name, and which a record that holds it already gives back, and that root is what it gives back.
  What the record holds of an act made again keeps that act from the outside: a done that is its first answer the journal says at once, and an act that the outside started the journal holds with no fact, since it cannot run it, so no ear of the outside hears that act, and a chain with a source which asks again what its origin asked is answered from the record too.
  The record a life was opened from, from which the journal answers what it holds of an act, so that an act the World did once is done no more.
  The journal: an ear of the engine, which hears everything and keeps every act that no act of the life made and every fact that no act of the life said, the act it is about before it, and nothing that a generator an act brought to life made or said, nor anything it says itself, since a later life makes again everything that an act made or said and brings that generator to life again, and what the journal says the record holds already.
  A read the operator makes is kept like any other act of the operator, since what it reads may change, and a later life makes it again at its place.
  What it keeps it says, so that the World holds the record and the journal alone says what belongs in it; a World that is durable keeps what it is told, one that is not keeps nothing, and either way what the World holds is what the life after it is given.
  Given at its birth what the World kept of an earlier life, it says those entries again in the order it was given them, each once every fact said before it has been heard: an act whose maker is neither an act nor an ear that boot was given it makes again through its verb under the site of its maker, once the chain it is on has been made again, since that maker makes nothing again of what the record holds; an act whose maker is an ear that boot was given or an act it makes not, since that ear, or the word of that act, makes it again when it needs it; and any other fact it says once the act it is about has been made again, so that its controls and its dones land where they landed.
  An entry whose act this life has not made again when every fact said before it has been heard, which for an act whose maker is neither an act nor an ear that boot was given is the chain it is on, the journal steps over, since this life will not make that act at that place, and it keeps the name of that act taken, so the entries after it go on and their acts keep their names.
  The journal says the whole record again before boot returns, so no entry waits for an act that a host makes after boot.
  One said again takes the name it had, since a later life makes its acts again in the order of the record, and the life counts each kind on from there, so that no later act takes a name that is taken.
  A boot is a life; a second boot is a second life, and the first is gone.
  The names operator and journal are the life's own, and boot refuses a generator of the outside under either name.
  A life settles an await of its acts from outside a run in the loop it is opened in, so boot outside a running loop raises before it makes anything.
  An act whose maker is neither an act nor an ear that boot was given is said again through its verb, with the words the record holds and the chain it names, so its ear is the verb's, a host finds it made when boot returns and makes it not again, and a show or a filter it was given is not said again, since the record holds none.
  A later life says an act whose maker is neither an act nor an ear that boot was given again only through a verb in the globals of its chain, and an act whose kind no verb binds is a drift.
  The journal says again a done that is the first answer of an act when the act is made again, and every later fact of it but a started at the place where the record holds it.
  What the record shows started and not done when boot returns is pending: the journal holds it from the outside until a wake that this life says.
  """

actor: str
"""actor is the default actor of the chain, bound from the standing.
The program rebinds actor like any name, and the last binding wins.
"""
raised: BaseException | None
"""raised is the exception object that the last rung raised, rebound at each raise.
raised is None at the birth of the module of a chain, so a word reads it before any rung raised."""
