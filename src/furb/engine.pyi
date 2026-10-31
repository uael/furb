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
LARGEST: Final[int] = 524288
"""LARGEST is the largest number of bytes that a read tells of the lines that its show picks."""
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
An act is on the chain or the thread that the verb names, and a thread is a prompt.
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
type Ear = Generator[Saying | set[str] | None, Fact]
"""An ear is any generator of that shape, so the World and the Kernel are ears, and boot takes an ear of the outside under any name it is to hear by.
An ear that yields a set of names waits for the facts that name one of them: by the kind of the fact, the act that it is about, or the kind, the maker or the chain of that act, or the chain of the maker of that act.
An ear that yields nothing, or an empty set, waits for every fact.
Every ear hears every pause, wake, cancel and close, whatever it waits for, so a control reaches each act that it is over.
The World, the ears of the outside that serve the machine, one or many, hears every fact that it waits for, every act that no ear before it took, and every chain at its birth: it answers a stand, a clock, a chance, a cd, a read and a write of a path nobody of the engine serves, resolved against the working directory of the chain, or against the scratch directory for a path of scratch://; it takes a command, asks it whether it is merged, answers a write of its doors and feeds it what a write of its stdin gives, and ends it at its timeout and at a cancel; it takes a wait and a prompt to the operator, which it shows; and it takes a reply, which it answers with the turn of the model.
The World performs any fact that an extension defines and that the World knows.
The opening of a life gives the World the scratch directory of its record, which every chain of that record shares, or, for a life that keeps no record, a scratch directory of its own, which goes when that life ends.
A later life on a record finds its scratch directory again, and when the directory is missing, as on another machine, the opening makes again in it each file that a write of the record made, in the order of the last write of each, with the text that the journal kept, and no file that a command made; the opening of a new record starts with an empty directory.
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
  Every generator hears every fact that it waits for and has not heard, in order, until none is left, and each act as act puts it.
  While one speaks nobody hears, and whoever spoke has everyone hear when it is done, so the facts of one speaker stand together in the log, and no one is having everyone hear while another is.
  A done said of a question that has no outcome yet fills its outcome, and a later done of the same question fills nothing.
  """

def act(kind: str, on: str, ear: Callable[[str], Ear] | None, *words: object) -> Act[object]:
  """The way to make a question: it takes a name when it is made, it is put to the ears, its ear, when it has one, is brought to life under that name and given the name, and the name is given back, which is the act to whoever holds it.
  An act is put to every ear of the engine that waits for it, the acts first, and a busy one hears it once it is done speaking, at its place among what was said.
  Its own ear is born after the ears of the engine heard it, so the acts that ear makes stand after it.
  It is then put to the ears of the outside in turn, until one takes it, and no ear of the outside after that one hears it, a chain apart, which every ear of the outside hears at its birth after its own ear took it, so an ear of the outside plays on a chain before the call that made it returns.
  An ear of the outside that is busy when its turn comes and waits for the act hears it once it is done speaking, and the act is refused only when no ear of the outside took it by then.
  To take an act is to say a started or a done about it: a done settles it now, and a started says that its done comes later.
  An act that no ear takes and that the record does not hold is refused: the life says it done with a refusal that names its kind.
  An act said: it is begun, and what the call gives is its name, which is awaited for what the act comes to.
  An act said twice under one name is one act, and the second saying brings no second ear and gives the name back.
  Two acts that say the same words under one name are one act.
  The engine refuses an act said from outside a run that names no chain, a chain apart.
  The chain an act is on is the chain named to the call, or the scope of the one that made it when the call names none.
  The ear of an act is given the name of the act and hears every fact said after its birth that it waits for, and it speaks by yielding a saying.
  An act carries the words of its kind, which are the plain arguments the verb was given, in the order of the verb, and a show or a filter is none of them.
  """

def drive(g: Ear, name: str) -> None:
  """The other way to speak: a generator is brought to life under a name, and from then it hears every fact that it waits for and says its own.
  One that returns is over and lives no more, which is how a thing that watches for one fact alone is dropped the moment it hears it.
  One that raises while it hears is broken the same way: the next fact it is given finds it over, and what went wrong goes to the one that spoke.
  A generator brought to life under a name and nothing more: it hears from the tip and runs to its first wait, and one born while a fact goes round hears from the next.
  It lives until it returns, and an act that hears nothing more returns at the first fact it hears after its own end.
  A generator that yields a saying is given the fact as the bus said it, one that yields a set of names waits for the next fact that names one of them, and one that yields nothing waits for the next fact said.
  """

def transcript(on: str = "", of: str = "") -> list[Fact]:
  """transcript gives the facts on a chain, each of which the life adds when it is said, so a chain reads at once what it said itself.
  transcript(on, of) gives the facts on a chain whose kind is of or that of said, in order, and the view of pause holds the wakes too.
  transcript gives a new list at each call, so a word that changes the list it was given changes no transcript.
  The transcript is the whole state of a chain: its module, its program, its working directory and its turns are read off it, and the standing is read off the transcript of the root.
  The name of an act gives the facts on its scope, and a name of no act gives no fact.
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
  """grep(pattern) is the show of the lines that the pattern matches."""

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
  """A read: whoever serves the path answers it with the text of it, which the read tells by the lines its show picks.
  A read that the World refuses raises Refused in the caller.
  The World refuses a read of a path whose first part is the name of an act, since that path is a door that no ear of the engine served and never a file, and the refusal names the chain of that act.
  The engine judges no scheme, so a path of an unknown scheme goes to the World too.
  The World serves the scratch directory as scratch://: a read of scratch://plan.md gives that file, and a read of scratch:// or of a folder of it gives the name of each entry of that folder, one on each line, each folder with / at its end, as ls does.
  Each text and each refusal of scratch:// names its path under scratch:// and never the place of the scratch directory on the machine.
  A path of scratch:// that leaves the scratch directory, by .. or by a link, is refused, and so is a path that starts with scratch: and not scratch://, with a refusal that names scratch://.
  The World gives a file whose every line ends with CRLF with LF at the end of each line, so a text holds the same lines on every machine, and a file of other line ends as it is.
  On Windows the World reads a path in the form that the sh of Git gives it as Windows names it: /c/x is C:/x, and /tmp/x is scratch://x, since the sh names the scratch directory /tmp.
  read is given a path and a show.
  read gives a Text.
  A text without a show is told as HEAD, which is the span of its first 2000 lines.
  A read of the name of a prompt gives the program of that ladder, the words of its rungs in order.
  Whether a name is one of the prompts a chain has heard on itself, which is what the doors of its prompts serve and no other path, a path of no name being none of them.
  A prompt is the door of the program of its ladder, so a read of its name gives the word of every rung of it in order, the words the gate refused among them, which are none at all for a prompt that ran no word.
  A read of a door that a rung of that ladder says leaves the word of that rung out, since a rung is no part of the program it reads.
  One made from inside an act tells itself under its own name, with its path and what it was answered, about itself, on the chain it is on; one made from outside an act tells nothing, and neither does one whose show is hidden.
  A read answered with what is no text gives that value, and tells it as python shows it.
  A read from inside an act that would tell more than LARGEST bytes of its text raises Refused, which names those bytes and LARGEST, and tells nothing, and a read from outside an act, which tells nothing, is never refused for its size.
  """

def write(text: Text, on: str = "") -> Text:
  """A write: whoever serves the path of the text takes its content.
  write is given a text, and gives the text as it is on disk after the write.
  A write that the World refuses raises Refused in the caller.
  The World refuses a write of a path whose first part is the name of an act, and writes no file, since that path is a door that no ear of the engine served, and the refusal names the chain of that act.
  The engine tells a write of a text by its header and its path, with the lines that differ from what the caller asked when some do, and a write that a door answers with a value by that value.
  A write takes no show, since what a write would show the word of the model already said: it tells its header and its path, so the turns show each write that landed, and the lines of what came back that differ from what it asked for, so a write that the disk took as it was asked tells no line.
  A door that answers a write with more than it was asked for tells each line of what came back that differs from the line it was asked for at the same place.
  A write to the door of a prompt edits the program of its ladder, so the door and the verb are one act.
  The chain answers a write of the door of one of its prompts with the text it took, and makes its ladder from it.
  The chain makes the rungs of a write of a door from its text less the line breaks at its end, since a read of the door joins the words of its rungs with one, so an append to the door makes a rung of what it adds alone.
  A write of a door that a rung of that ladder says leaves the word of that rung out of the ladder, so the door shows it no more.
  A new file is a write of a Text made of its path and its content.
  A write of scratch://plan.md puts its text in that file of the scratch directory, and makes the directory and the folders of the file first when they are not there.
  A write to a file whose every line ends with CRLF ends each line of the text with CRLF, so the file keeps its line ends, and gives the text with LF, as a read does.
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
  The turns of what a chain has heard: every tell stands as a paragraph of its notes, a run with no donor stands as a paragraph of its word, and nothing else stands at all.
  The turn a model was answered with closes the turn of the operator and stands as the turn it is, and a text stands by the lines that the show of the one that tells it picks.
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
  """program gives the program of a chain: the word of every rung that runs on it since its last module, as python, each under the name of the rung that the word first ran in, which is the rung itself, the rung that a rung which retells retells, or a told rung, in order, as the runs of its transcript say."""

def standing() -> Standing:
  """standing gives what the chains stand on: the answer of the last stand that the transcript of the root holds, and an empty standing before the first.
  It reads the transcript of the root as it stands where the call is made, so a grant reads the window of an actor off the standing where the answer of its reply lands, and a later life reads at each place of the record the standing that the record held there.
  """

def stand(on: str = "") -> Standing:
  """stand asks the World what the chains stand on and gives the answer, which boot does on the root at the tip of every life.
  Only a stand on the root changes the standing, since the standing has one home.
  A stand from inside an act tells its answer under its own name, with the roster and the actor of the standing, as stand2_roster and stand2_actor, and no directory, which a cd tells, and no path of the doctrine, which the World reads in a turn of its own.
  A stand from outside an act tells nothing, so the stand that boot makes tells nothing.
  A stand that names no chain is on the chain of the act that makes it, as every verb, so the operator names the root, as boot does.
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
  The gate reads the word after the parts that the writings of a rung with no run hold too, so a rung that a word makes while its model writes it is read after what that word ran.
  A response that is not python is a finding like any other.
  Each finding names the line and the column of the word it is found on, and the rule, and quotes that line, cut to 80 characters, so a model reads what the gate read.
  A finding on a call of a function names the signature of that function, so a model reads how to call it.
  A finding on an await of a value that is no act says that a value which an await gave stands in its name already, so use the name and await it no more.
  A word that is not python and that holds a fence of markdown, the markup of a tool call or a sentence of prose finds the first such line and what it is, and that a reply is python and quotes only, with no fence, no markup and no prose, so nothing of this word ran: delete each such line.
  A word that is not python and that holds a mark of a quote inside a line, as write(Text(path, <s:name>, finds the first such mark, and that a quote opens only with <s:name> at the start of a line and closes only with </s:name> at the end of a line, so nothing of this word ran: write the quote on lines of its own before the call, and give the call the name.
  A word that is not python and that leaves a bracket open finds the line and the column of the innermost bracket that stays open.
  A word that is not python and that holds a string of one quote that a newline ends finds that a string of one quote ends at the end of its line, so write a text of several lines as a triple quote, or as a quote <s:name> on lines of its own before the call, and nothing of this word ran.
  A word that is not python and that starts with a shell command finds that the line is a shell command, so write it as await bash("..."), and nothing of this word ran.
  A word that is not python on a line that starts with a lone = finds that the line starts with =, so its binding has no name: write the name before =, as in got = 1, and nothing of this word ran.
  An open mark of a quote that has no close mark is a finding of its own, which names that mark and the close mark it wants.
  An open mark that stands inside a quote of its own name is a finding of its own, which names that mark and the line of the quote that holds it, since one close mark ends both.
  A word that binds a name that the transcript binds, the name of an act of its chain or a name of a value that a tell of that act binds, as bash12_stdout, finds that the transcript binds that name, also for an act that the chain did not tell yet.
  A word that binds a name to a value of another type than the one that a rung before it declares for that name, as a def, a class, an import or a name with a type does, finds the rung that declares that name, so the model gives the value another name.
  The gate gives no finding when the gate accepts the rung.
  The journal keeps what the gate found, since the gate is of the outside, so a later life reads the same findings and asks the gate nothing again.
  """

def cd(path: str, on: str = "") -> str:
  """A cd: the paths of its chain resolve against the directory it came to from then on, and it does nothing else.
  cd completes at once and gives the new working directory.
  The World answers it with the directory that its path names, resolved against the working directory of the chain, and refuses a path that names no directory, so a refused cd moves nothing.
  It is a question and no fact, since a fact a running word says is heard when the word yields, where a question is answered at once, so the paths of that word resolve against the new directory from then on.
  cd tells the directory it came to.
  A cd to a path of scratch:// is refused with a refusal that names scratch://, since a chain cannot stand in a directory that has no path.
  """

def cwd(on: str = "") -> str:
  """The working directory of a chain is the directory that the closest cd back in its transcript came to.
  A chain whose transcript holds no cd has no working directory, so cwd gives an empty path, and the World resolves the paths of that chain against the directory of the standing.
  The World resolves the path of a read, a write and a command against the working directory of the chain, which it reads, and the path of a cd, a read or a write that is ~ or starts with ~/, or with ~\\ on Windows, against the home of the user, and one of scratch:// against the scratch directory.
  The World writes each path under the home of the user with ~ in place of the home, as ~/a.txt, in each text, each cd and each standing that it gives, and every other path as it stands, so a path that a model reads names no home.
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

def pause(id: str, why: object = None) -> None:
  """A pause: while it stands, nothing it is over hears, and what is said meanwhile waits for the wake.
  A control from outside the chain of an act that is not over tells a header of its own name, so a model reads what another did to its work.
  pause is given the id of a pending act or the id of a chain.
  A pause carries its cause, None when none is given, as a close carries its value, so a pause from outside the chain of its act tells that cause under the word value.
  A pause stops no reply in flight: the reply returns.
  A paused chain goes quiet as its in-flight work returns.
  A kind a pause stops: it starts its ear, and while a pause over it stands the ear hears nothing, and at the wake it hears the wake and then everything that was said meanwhile, in order.
  An act made in that time it hears at once, since an act is put to the ears while it is made and to no ear after the one that takes it, a chain apart, so a paused rung takes the wants of its run.
  A pause stands over what is made after it, until the wake.
  A control is on the scope of what it is over, so it takes no chain of its own.
  A pause of a name of no act of the life raises Refused, which names it, and says nothing.
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
  A wake of a name of no act of the life raises Refused, which names it, and says nothing.
  A wake that the journal says again says at once each next entry of the record that answers a pending act it is over, as the outside said them while it heard that wake, before any other fact of the life.
  A wake that this life says past the tip, and not one that the journal says again, puts every pending act it is over on to the outside, so the World takes each wait, prompt to the operator and reply of them and closes each command of them as interrupted, and a model reads the transcript as it grew.
  """

def cancel(id: str, why: object = None) -> None:
  """A cancel of that prompt reaches the acts that its rungs made on the chain with a source.
  A cancel is over the act it names and everything that act made, and each of them is done with CancelledError.
  cancel is given the id of an act and a cause, None when none is given, and says a cancel over it that carries that cause.
  A cancelled act completes with CancelledError, which carries the cause of the cancel when it has one.
  A cancelled prompt raises CancelledError to whoever awaits it.
  A cancel touches nothing else on the chain.
  The awaiter of a cancelled command raises CancelledError in its step.
  A cancel of a name of no act of the life raises Refused, which names it, and says nothing.
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
  A close on an act that is over tells nothing, and reaches only the words that still run under it.
  A close of a name of no act of the life raises Refused, which names it, and says nothing, so a caller that names a wrong act learns it.
  A close said from a word that names no act is over the prompt that made the rung of the word, and over the rung itself for a word its caller wrote, which answers no prompt.
  A close said from a word that retells reaches nothing and says nothing: it stops the word where it stands, so the rung is done with nothing and answers no prompt.
  A close of the prompt of the running word stops that word where it stands, as a raise does, and nothing after the call runs.
  """

def debug(template: Template) -> None:
  """What a word tells of itself as it runs: the text of a template, which debug makes as python makes the f-string of the same template.
  The engine tells what a step debugged.
  A debugged paragraph binds the text as rungN_debug under the header debugged, as #rung5 debugged and rung5_debug = 'i=3 7', and a text of more than one line as a quote.
  A raised header and a debugged header stand at the place in the run where they happened.
  The transcript holds between the entries what the run of each word raised and debugged.
  debug is given a python template string.
  debug makes each interpolation as an f-string makes it, with its conversion, !r, !s or !a, and its format spec, so {x=} makes the expression, an equals sign and the repr of the value.
  Each debug of a rung binds rungN_debug to its own text, and a later one binds it again, as python binds a name again, so the name holds the last text and no text is told twice.
  debug binds nothing in the module itself, since the told rung binds what its paragraph tells, rungN_debug among it.
  debug tells nothing but the text of its template, and nothing at all when that text is empty.
  debug enters nothing in the record.
  It is no act and it enters no record, and it stands in the turns at the place in the run where it happened.
  Outside an act there is nothing to tell of, so it tells nothing, as a tell outside a run.
  A tell is on the scope of the act it is of, so debug takes no chain of its own.
  """

def wait(seconds: float = 0, *acts: str, on: str = "") -> Act[str | None]:
  """A wait: the World takes it and says it is done with nothing when its seconds have passed, and it is over then.
  A wait stands in no turns, since a wait is no work of a model.
  It tells nothing, since a wait is no work of a model.
  wait(seconds, *acts) is done at the first of two events: with the name of the first of its acts that is done, or, of those that are done already when the wait is made, the first in the order it is given them, or with None when its seconds pass first.
  The ear of a wait says it done when one of its acts is done, and the World stops its timer when it hears that done, as when a control ends it.
  """

def rung(word: str = "", retells: str = "", to: str = "", on: str = "") -> Act:
  """The run of a word on a chain: a word its caller wrote, which it tells, since nothing else did; or, with no word, a turn of a model, which its chain asks for at the turn it gives it and which the World answers, of which it tells nothing, since that turn stands as the turn it is.
  A rung is an act: the run of one word in the globals of its chain, which the chain has the Kernel run.
  The engine tells what a step raised.
  A step that raised nothing and debugged nothing tells nothing.
  The open of a rung with a word tells that word as a command tells its command, bound as rungN_word, so no word that ran runs again.
  The header that tells the word of a rung names the maker of that rung, as #rungN by operator, so a turn says who made a word that it did not ask for.
  A rung with no word tells nothing where it is made, since the chain tells it as the last line of the turn it asks for it with.
  A rung with no word and no actor takes the default actor of its chain when it is made, and writes it into its actor word, so its reply and its ledger read the one actor.
  A rung with a word and an actor is a turn of that actor that its caller wrote: it answers its own reply to that actor with the turn ("assistant", word, None, None) before it says its word ready, so the turns show the word as the turn of that actor, the rung runs it as its step, and no World asks a model for it.
  Such a rung tells no header of its word, since its turn shows the word.
  A rung with a word waits to run it while the first rung with a word and an actor that is open on its chain is neither itself nor one of its makers, so the turns of the words that their callers wrote stand in the order that their rungs were made, each after what the one before it told, a word that no such rung makes runs after them, and such a rung that one of them makes takes its turn within the turn of its maker.
  Its turn holds no usage, so a grant counts no cost of it and no share of the window.
  A later life makes that reply again in the ear of the rung, with its name and its turn, so the record replays it with no drift.
  The raised paragraph binds the exception as rungN_raised, as python shows it, which says its type and its message.
  A word that raises a CancelledError tells cancelled and no raised header, since a cancel is no fault of the word.
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
  rung is given a word and runs it on a chain in the globals of that chain.
  The gate reads the word its caller wrote like any word.
  A rung with a word completes with what that word raises, and with nothing when the word runs to its end.
  A close ends the rung that runs in a prompt at its next await.
  A chain with a source and its origin share the one act, the record's.
  A chain with a source awaits what its origin started, through the name of the act.
  It says its word may run as soon as it holds one, whichever way that word came, and what the chain makes of the word is the chain's.
  The chain has the Kernel begin it in the module of that chain, and the run carries it forward at the done of every act the word waits for, so that the engine owns the order of it.
  It is done with what the word gave, nothing for a word that ran to its end and the exception for a raise, which it tells with its type and its message, or as cancelled for a CancelledError, which its close then holds none of.
  Of the acts its word made and read at once it tells nothing, since such an act tells of itself or not at all.
  An answer with no text is a word like any other, so the gate reads it, the run gives no value, and the model is asked again.
  A replay makes a rung of its own retelling each rung of the donor it keeps.
  A replay makes no rung for a word that is empty, since a rung with no word asks a model for a word.
  What a rung that retells asks is named under the one it retells, so the life answers it with what it answered then and the World is asked nothing twice.
  A rung that awaits an act nobody settles waits until a cancel ends it, and holds nothing else of the chain.
  A rung that retells says each question it makes as the rung it retells, so the question it makes at a place is the one that rung made there.
  A rung that retells names the rung that the word first ran in and never another rung that retells it, so a second replay makes the same acts and asks the World nothing twice.
  A rung that retells is done with nothing when its word answers, runs to its end or is cancelled, whatever the Kernel makes of the word, and with what that word raised.
  A rung takes its own act, since the engine is the one that runs it.
  A rung with no word holds the word of its model when the reply for it is done, and it is done with the refusal a reply came to.
  A rung with a word that it does not retell says its word ready only while the one that made it is not paused, so a paused prompt runs no word that a write of its door gives until the wake.
  A rung that retells says its word ready only once the rung that it retells is over, so no word runs twice at once, and the copy of a word that ran on through a replay finds each act of that word done and binds what it bound in the new module.
  """

@overload
def prompt(shape: None, message: str = "", to: str = "", on: str = "") -> Act[None]:
  """A prompt: it makes the rung of one turn of its model, makes another when no rung that it made runs, and is done with the value, so a rung whose word is refused and a rung whose word raises are asked again alike.
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
  The actor that a prompt leaves unsaid, and that its thread leaves unsaid too, is the default actor of the chain when each rung of the prompt is made, which that rung writes into its actor word, so a rung asks the actor the chain stands on then, a prompt that names an actor asks that one at every rung, and every life asks the actor the record holds.
  A prompt to a model runs in steps until the prompt completes.
  The binding of a prompt gives the shape as python shows the expression, as prompt1: Act[int] = Act('prompt1').
  A rung whose word closes nothing ends its step, and the engine asks the model again.
  The completion of a prompt cancels nothing under the prompt but the words it ran.
  The operator prompts on any chain, by the id of the chain.
  A rung prompts on any chain, by the id of the chain.
  A prompt to the operator of a shape that is not None completes when the operator closes the prompt.
  The response of a prompt on a chain with a source comes to the act that the caller holds.
  It is the ladder of its rungs, and its name is the door of the program of that ladder, which holds the word of every rung of it for as long as the chain lives.
  A word written to its door is a rung of it, which answers it as the word of its model does.
  A prompt to the operator asks no model: the World takes it and shows it, and it waits to be closed, but for a prompt of shape None; in a later life, one the record shows open is shown again only at a wake, and one the record shows closed is shown no more.
  A prompt to a model takes its own act, since the engine is the one that asks the model.
  Its close tells what closed it from outside, which the one that closed it says, and nothing of what its rung gave, which the rung has told.
  A paused prompt makes no rung until the wake, and a cancel of it is over its rung too, which ends itself.
  The World closes with a refusal a prompt it cannot put to the operator; which shapes the operator answers is the World's law.
  The World closes a prompt of shape None to the operator with None as it takes it, since that prompt asks nothing: the operator reads it as a notice, and no word that awaits it waits for the operator.
  The shape left unsaid is None, which the acknowledgment uses, and any value responds to it.
  A prompt takes any shape, which a close is read against as python reads an instance: of the shape, or of the origin of a generic one, a Literal as one of its values of the same type, and a union as any of its members, each read the same way.
  A prompt carries the name of its shape as a word, and takes the name as well as the shape, so the journal makes it again.
  The name of a shape is the word a chain says it by, so a shape that holds a class of the engine or of the chain names it as the chain does, under no module.
  The acknowledgment carries no shape and a message that names the act and what happened to it, as prompt3 done.
  A done whose value is None makes no acknowledgment, since it gives the model nothing to read.
  The acknowledgment is a message on the thread that made the rung that made the act, while that thread is open, so the actor of that thread reads it at its next step in that thread.
  When that thread is over and the act did not fail, or when that rung answers no prompt, the chain prompts nothing, and what the act told waits for the next advance, which shows it.
  A prompt on a thread is on the chain of that thread, and goes to the actor of the thread unless it names one.
  A prompt on a thread tells the thread it is on under the word thread, as prompt4_thread = 'prompt1'.
  A prompt of None on a thread that is open and that a model answers is a message of that thread: it tells its message, makes no rung, and is done with None at once, so the model of the thread reads it at its next step.
  Every other prompt on a thread is answered in steps, with a ladder and a door of its own, and on a thread that is closed it opens the thread again, so the model of the thread goes on with the whole transcript of its chain.
  A thread is a prompt and the prompts on it, in order, and the record keeps each done as it landed, so a later life makes each prompt on a thread again at its place, as a message or as a prompt in steps as the thread stood there.
  A prompt that a rung of a thread makes is a thread under it, by its lineage.
  The turns of the chain hold the result of the command that the acknowledgment names.
  A cancelled result is no orphan.
  When an act that a rung of the chain made is done while the word of that rung does not run, no word of the chain awaited it, no reply has shown it, no rung of the chain waits for a reply or ends the step of a prompt that is open, and no word of the chain is running, the chain acknowledges it, though after a word that the operator or an outside ear gave it waits for the next fact on the chain.
  The chain acknowledges too a failure of an act that a rung of the chain made before the run of that rung, as while its model writes its word, and a failure of an act but for a cancel opens that thread again when it is over, so an error asks the model again even after that word closed its prompt.
  The chain acknowledges no done that a word of the chain caused with a close, since the model that wrote that close knows it already, and it acknowledges a done that the chain itself caused, as the close of a rung that it refused.
  A pause stands over the close that answers a prompt too, so what a word gave waits for the wake.
  A prompt tells its message and its binding where it is made, as every act tells its open, whether a model or the operator answers it, and a prompt to a model tells them once no rung with a word and an actor is open on its chain, so its message comes after the turns that their callers wrote, as the setup of a chain.
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
  The open of a chain with a source binds its label and its source.
  The entry that opens a chain with a source carries its label, after the prefix.
  chain gives the new chain, which never completes.
  boot gives the root as an act of Never, and the root never completes.
  The engine makes a chain in one way: by running its rungs.
  In a chain with a source the rungs of the origin that are over at that source run again in the module of the new chain.
  A chain given a source stands on that one: it retells the words of it as they stand, each rung of it retelling a rung of that one, so that it makes the same acts and shares them, and what it holds of the transcript of that one is what its filter kept, though it runs every word all the same, so what it holds bound is more than its turns say.
  A prompt to an actor its roster does not hold it closes with the refusal.
  A chain that the word of a rung opens is a scope of its own: its words are on itself, though the chain fact itself stands on the chain of the rung that opened it; and when that rung is retold, the word makes the same chain, since a rung that retells another shares the acts it makes, so a word that opens a chain opens it once.
  The engine refuses a prompt to an actor outside the roster.
  The chain holds the control it says itself, which tells nothing, since a control from inside a chain tells nothing to it.
  The engine binds the exception of every raise but a CancelledError in the globals of the chain, under the name raised.
  The engine binds the exception again at each raise but a CancelledError.
  A chain: its module, which is the engine itself, named for the chain, since everything the file defines is the model's to call and nothing of it is bound to one chain, and what the operator would add to it, it makes a rung of, which binds it, stands in the program and is said again in a later life, a fact said from a run being on the chain of that run, and a word that wants a chain of its own giving the name of its own as the source.
  A chain is chainN whether boot or chain opened it, and the root is chain1.
  A chain is an act of Never, so the chain never completes.
  The turns of a chain tell the acts of the operator.
  The transcript of a chain is the facts on it, in the order they were said.
  Nothing leaves a transcript once the transcript holds it, but by a prefix said on its chain, which takes the place of what the transcript held.
  An assistant turn keeps the role assistant in every chain made from the chain.
  The engine adds the acts that caused a kept act to what the filter kept.
  The globals of a chain whose filter is a take that is not inside hold the bindings of the skipped words still.
  A chain with a source made after the rung whose word defined a door has the door too.
  The globals of a chain are those of a module named by the id of the chain.
  Every name that the file defines is in the globals of a chain.
  A chain holds whole every act made on the chain, the rungs it makes itself among them, which tell nothing.
  A chain holds every fact on it, and an act on another chain is that chain's.
  A prompt that a step of another chain made reads nothing of that chain.
  The word of a rung rebinds the default actor like any name, and so does an answer of a stand that changes the actor of the standing, and the last binding in record order wins.
  The transcript of the root begins with the open of the root, when its standing names no doctrine, since no setup comes then.
  When its first standing names a doctrine, the root tells the bare paragraph chain() at once, and tells its open once no rung with a word and an actor is open on it at the done of a rung on it, so its first turns are the setup that the World plays on it, then the turn of its stand, and the turn after it holds what the stand told, then the open.
  The chain runs no paragraph chain() as python, since that paragraph tells no call.
  A chain with no source holds first the setup of the root, each fact of the root from its paragraph chain() up to its open but the facts of each act that an ear of the outside, the operator or the root itself made on the root after the act setup that the World makes on it, as the rung of the stand of the root, and of each act under one, so every such chain holds the turns of the setup that every chain shares with the same names, and the turn of its stand and its own open come after them.
  A chain with no source whose standing names a doctrine plays on itself, once no rung with a word and an actor is open on it, the rung of stand() and a cd to the directory of the standing, with that word and its default actor, the root at the done of a rung on it after its setup and every other such chain at its birth, so a prompt on it waits for that rung, and it tells its open once that rung is done, so its last turn before its first prompt tells where it stands.
  A chain that hears a prefix that another said on it tells no open of its own and plays no stand at its birth, since the turns of the prefix hold them, as the turns that a checkpoint carries.
  A chain with no source runs again each word of the setup that it holds, as a chain with a source runs the rungs of its origin, so the words of the extensions bind their names in its module and the setup tells nothing again.
  A chain with a source holds whole the setup that its origin holds, whatever its filter, and its filter applies only to what came after that setup.
  The transcript then holds the prompt of the operator, and the replies of the model, each with its turn.
  The engine reads its own names through the globals of the chain.
  The engine uses a rebound name from the next use on.
  No close reaches a chain, since a chain never completes.
  To await a chain never returns.
  A chain has a globals dict and a working directory of its own.
  A chain with a source holds the acts it inherited from that source, as the filter kept them.
  A chain with a source holds whole each fact of its origin about a name that no act of the life holds, as a turn that its origin carried from another record.
  The steps that an inherited prompt takes after the point enter the transcript of its owner alone.
  What a chain binds is its own, and a chain with a source is how a chain gets isolation.
  Two chains from one source hold the same values, since both ran the same rungs.
  A chain with a source inherits the default actor with the globals of its origin at that source.
  A chain with no source stands on the default actor of the chain it is on when it is made, which for a chain that a word opens is the chain of that word, and on the actor of the standing when it is on no chain.
  A chain with a source that holds an act reads the close of the act in its transcript.
  A chain with a source awaits or peeks an inherited act as it likes.
  The engine does not wake a chain with a source for the result of an inherited act.
  The globals of a chain with a source may hold more than its turns say.
  The transcript of a chain with a source holds the entries up to that source first, then the entry that opened it.
  The acts of the prefix of a chain with a source keep the ids they had on the origin.
  The globals of a chain with a source are those of a module of its own.
  The objects of a rung of a chain with a source are that chain's own, made again, and only code is shared.
  The globals of a chain with a source are what the rungs of the origin that are over at that source bound, whatever the filter kept.
  A chain with a source inherits the words its caller wrote with its prefix, wherever its source.
  A filter that skips a rung its caller wrote keeps it out of the turns of the new chain.
  A chain with a source holds the classes that its origin defined before that source.
  A chain with a source reads nothing that its origin did after that source.
  A chain with a source retells no rung of its origin that is not over at its birth, so the word of that rung goes on in its origin alone, and no copy of it waits for it or runs it a second time.
  What a rung of a chain with a source binds lands on that chain and not on its origin.
  The origin holds the chain fact of a chain that a rung of it made, and nothing of that chain's own.
  Two names of the module are the chain's to bind: the actor it stands on, and what the last rung that raised raised, so that the word of a rung reads what the word before it came to.
  It holds the transcript of its origin first and tells its own open after it, and the rungs it runs again tell nothing, since every word of those stands in what it inherited.
  It hears no control that ends it, since nothing that happens to a chain ends it, and a control over a chain is over the acts on it, which end themselves.
  Every done it holds answers a question it holds, since it holds every question made on it, the ones it made itself among them.
  What a chain with a source holds of the transcript of its origin: what its filter kept, everything that made what it kept, the reply of every rung it kept, which cuts its turns where the origin asked its model, and the open of the origin.
  The filter is given every act, what the operator asked among them, since what the operator asked of a chain no rung of it says again.
  What a chain said of itself it keeps whatever the filter says, so the open of the origin stands in the new chain.
  The source of a chain is a chain, by its name, and means the transcript of that chain as it stands.
  A source that names no chain of the life refuses the call in the caller, and no chain is made.
  A chain with a source reads what its origin stands on, as it stands.
  The chain retells the rungs of its origin through the program of the origin, and it makes the rungs it retells itself, so they tell nothing.
  A replay makes the rungs of a chain again from its donor: it keeps each rung of the ladder whose word is the next part of the words it is given, it makes one rung of what is left, and every other rung of the ladder is gone.
  A replay makes the module of the chain again, as it was at its birth but on the standing the chain stands on then, and makes its rungs in that one, so what a word it drops bound is gone, and a word that runs while it happens ends in the module it began in.
  The donor of a replay is the rungs of the origin that are over at the birth of the chain for a chain with a source, and the rungs of the chain as they stand for a write of the door of one of its prompts.
  A write of a door gives the words of that ladder alone, so every rung of the chain that is no rung of that ladder stands as it did.
  A write of a door cancels each rung of that ladder that it drops, but the rung that writes, so the plan that it replaces ends wholly: a word that waits ends where it waits, with every act under it, the rung that writes among them when it answers that word, and a word that is done ends each act that it made and that is not done.
  A write of a door that drops no rung that ran, since the gate refused each rung that it drops, keeps the module and every rung of the chain as they stand, and makes only the rung of what is new.
  Before every reply the chain tells the last paragraph of the turn, whose header says what the answer is for, as #rung5 advance on prompt1, which names the one that made the rung.
  The advance binds the share of the window that the last reply on the chain read, as rung5_share = 0.25: the tokens that reply read over the window that the roster gives its actor, to two places, and no share before a reply that read a token.
  The advance binds each act that a word of the chain awaits and that is not over, as rung5_waits = ['bash3'], and no waits when no word awaits, so the model sees which acts the words that it wrote before still wait for.
  When nothing was told on the chain since the word of the last rung that the chain did not make was ready, the chain tells that rung under the header told nothing before the advance, so the model sees that the word showed it nothing.
  While a word of the chain awaits an act that is not over, the chain advances no rung after a word that told nothing, so it asks no reply until a fact is told on the chain.
  The chain holds the text of each paragraph told on it, in order.
  At the advance of the next rung, before it asks the reply, or at the first ready of a rung that it does not advance, the chain runs that text as the told rung of that rung, and holds nothing more.
  Each act that a told paragraph opened and each value that it told is bound for the word of that rung, whoever wrote it.
  The gate never reads a told rung as a word, since the engine wrote it.
  The turns do not show a told rung, since the turn shows its python already.
  A told rung is no act, and the record keeps neither it nor its run.
  A later life hears the same paragraphs and runs the told rung again.
  A told rung takes the name of the rung whose advance or first ready runs it, with _told after it, as rung5_told.
  A rung makes at most one told rung, at its advance or at its first ready, so no two told rungs share a name.
  The chain advances no rung that is over, so it asks no reply for a rung that a close or a cancel ended before its advance.
  A statement that the model of a rung writes runs after the told rung of that rung, so it reads each value that the advance of that rung bound.
  The chain takes its own act, since the engine is the one that runs it.
  The chain asks the model of a rung for its word by a reply it makes under the site of that rung, whose one word is the actor.
  When an act that a rung of the chain made on another chain is done while the word of that rung does not run and no word of the chain awaited it, the chain tells it about that rung under the header done, and binds what it came to under the word value, since its model reads no other transcript.
  When the chain of a prompt that a rung of the chain made on another chain pauses at a refused reply of that prompt, the chain tells that rung under the header paused and binds the refusal under the word why, and when the word of that rung does not run it acknowledges the pause, so a model sees that the prompt waits for a wake.
  """

def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act[None]:
  """A ceiling on a chain, in dollars, in the share of the window that one answer fills, or both: it holds the ledger of the chain from the moment it is made, the dollars of the answers since then and the share of the window the last one filled, and it tells the dollars of that ledger at each answer of a model, since the advance binds the share before the next reply, so no turn a reply has sent grows a line after it.
  grant on a chain puts a ceiling on it: dollars, a share of the window, or both.
  The engine enters a pause on the chain when a response carries the ledger to its ceiling, whose cause is ceiling.
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
  stdin: str | None = "",
  timeout: float = TIMEOUT,
  show: Show = TAIL,
  show_err: Show | None = None,
  on: str = "",
) -> Act[Exit]:
  """A command: its streams as they come, its exit, the door of its streams and of its stdin, and what it came to.
  bash is given one show for each stream that bash tells.
  The commands of the World run at the same time.
  bash is given a command, its stdin, a timeout, and a show for each stream, the plain words first, so the journal makes it again.
  bash gives the command, which is awaited for its exit code and its streams.
  The World runs the command in the working directory of the chain, which it reads.
  The World runs the command in a POSIX sh: /bin/sh, and on Windows the sh of Git for Windows, which it finds on PATH, beside the git on PATH, or in the folder of the programs of the machine.
  The World runs each command with TMPDIR set to the scratch directory, and TMP and TEMP too on Windows, which it makes first, so the command finds the files of scratch:// in $TMPDIR, and a program that reads TMPDIR, as tempfile of python does, makes its files there.
  The command runs until it ends, until its timeout, or until a cancel.
  The command ends once its shell exited and its streams closed, or once its shell exited and a short drain passed with a stream open, as when a process that the command started holds a stream, with the code of the shell and what its streams wrote.
  The World ends the command at its timeout.
  The command completes with its exit code and its streams.
  The exit code of the command is None after a timeout.
  A signal that ends the command gives the code 128 and the number of the signal, as a shell gives, so the code None says that the timeout ended it.
  A command that its timeout ended tells timed out and its timeout in its header, as #bash1 timed out after 5.0s, with what it wrote before.
  A stdin of text is fed whole to the command at its start, and then it closes, so the default, an empty text, gives a stdin that is empty and closed.
  Without a stderr show, the stderr of the command flows into its stdout.
  A command without a timeout has the timeout TIMEOUT that the file names.
  The World that takes a command answers it: it says it done with its Exit, the code and the streams it kept, since the owner of an act is the one that answers it.
  A stdin of None stays open, and the door bash1/stdin is writable while the command runs.
  The World feeds a command the text of each write of its stdin that it answers, and a write of nothing closes that stdin.
  The doors bash1/stdout and bash1/stderr of a command bash1 are readable while the command runs and after it: a read gives that stream of the Exit once the command exited, and otherwise the out facts of that stream, so the doors of a command that a crash cut give what it wrote before the crash.
  A wake on a chain with a source starts no inherited command again, since only the owner starts an act.
  The World takes it once: a later life never runs it again, and at a wake the World closes a command that the record shows started and not ended with a refusal that says the command was interrupted, so the model sees that outcome and runs the command again if it wants.
  With a hidden show it tells its header and its binding alone, and not its command and not its close.
  The chain that a read of a door of the command is on answers that read, while the command runs and after it, and the World answers a write of a door of it, so the ear of the command lives only until it hears the done of the command, and a cancel ends the command as it ends every other act.
  A read of a door of a command that an ear makes while the chain of that read speaks finds no ear of the engine that answers it, as a read of the door of a prompt finds none, since an ear that speaks hears nothing until it is done.
  It runs until it ends, until its timeout or until a cancel: the World is the one that ends it, at the timeout it reads off the act as at a cancel, since the World is the one running it, and the engine says nothing to make it.
  A pause stops no command: it runs on, the World says it done when it ends, and its ear tells its exit at the wake.
  Its close tells what its stdout shows, and what its stderr shows when that stream has a show of its own that is no hidden one, so a merged command tells no stderr.
  The stdout of a command without a show is told as TAIL, which is the span of its last 250 lines.
  The World refuses a write to the door bash1/stdout or bash1/stderr, and a write to bash1/stdin once the command ended or was cancelled, once its stdin is closed, as a stdin of text is from the start, or once a crash cut the command.
  The World asks a command whether its stderr flows into its stdout, and the command answers from what its verb was given.
  The World hears the bash itself, with the command, the stdin and the timeout, and no working directory and no show.
  """

@dataclass
class Text:
  """A text: its path, what stands at it, of which its lines are the lines, and the text it came from.
  The lines of a text derive from its content.
  Every edit of it gives another text, which came from this one.
  A text is equal to a text of the same path and content, and its repr shows its path and its content, whatever texts each came from.
  A method of a text that has the name of a method of str gives what that method of str gives, or is refused.
  """

  path: str
  content: str = ""
  @property
  def before(self) -> Text | None: ...
  @property
  def lines(self) -> list[str]: ...
  def edit(self, lo: int, hi: int, text: str) -> Text:
    """edit(lo, hi, text) gives a new text in which the lines of text take the place of the lines lo through hi, each with a line break at its end; an edit of lines that are not there is refused.
    The edits that say themselves are one edit each: a replace of a string, an append at the end, an insert before a line, and a delete of lines.
    An edit of lines, as an append, an insert or a delete, ends the last line of the text with a line break, so what it puts after a text that ends with none starts a line of its own.
    """
  def replace(self, old: str, new: str, count: int = -1) -> Text:
    """replace(old, new, count) gives a new text with the edit added, as str.replace gives with the same count.
    A replace of a string that the text does not hold is refused, and the refusal names the path and the string, as for an edit of lines that are not there.
    """
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
  An act is over when its done stands and lives until then; there is no other state, and a control over an act that is over tells nothing and reaches only the acts that it covers and that are not over, as control says, but a wake also reaches the ear of a command that a pause holds.
  The outcome of a cancelled act is the CancelledError it completed with.
  An act is dropped at its done, and so is its ear, but a lasting ear, which lives on until it returns, as the ear of a command does once it heard that done and told the exit.
  """

  def __await__(self) -> Generator[Self | Future[T], object, T]:
    """To await an act gives the value of the act when the act completes.
    An act is awaited from any chain.
    A rung that awaits an act reads the result of the act.
    A rung awaits an act and nothing else.
    To await an act raises the exception that the act completed with.
    To await a cancelled act raises CancelledError.
    A word that awaits a chain raises Refused where it waited, since a chain never settles and the word could go no further.
    A word that awaits a name of no act of the life raises Refused where it waited, since no done can settle it, and so does the operator.
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
  Its message names the act that drifted, the fact that the life made under that name and the fact that the record holds under it, which is the act or else its first answer, each cut to 300 characters, so a drift says the fact that differs.
  A drift breaks the journal, which keeps nothing more, and the life runs on with nothing kept.
  """

type Note = str
"""One thing a tell says: a paragraph, or the binding of an act.
A paragraph is what one fact that tells stands as in a turn: its notes, one after the other, and a blank line between two paragraphs.
The first line of a paragraph is its header: # and, with no space, the id of the act it is of, then what happened to the act, as #bash1 exited 0 or #prompt1, and no text that the act tells.
Every other line of a paragraph is python: a binding of each value that the act tells, and the binding of the act on its open.
The header of a paragraph names the act it is of by its id, what the act tells and a control over it alike, and the paragraph of a read, a write, a stand or a cd stands at the place in the run where it was asked.
The headers of the file are the open of an act, closed, done, exited, timed out, raised, debugged, refused, ledger, advance, paused, woke and cancelled.
A statement or a quote that a paragraph shows binds its name in the chain, and a comment binds nothing.
A paragraph binds each value that it tells under a name: the id of the act, an underscore, and the word that holds the value, as prompt2_message, bash1_command, read3_text or prompt2_value.
A paragraph binds a string of more than one line as a quote, and any other value as a statement of its repr.
A value that is None or an empty string is not told.
A quote that a paragraph tells takes one more underscore in its name for as long as its string holds the close mark of that name at the end of a line, so the first close mark after it is its own.
The engine applies a show before it writes a line, so a quote holds the lines that its show picked, as they are, and no number of a line.
A show applies to a text or to a stream.
"""
type Usage = tuple[int, int, int, int, float]
"""What one answer of a model cost: the tokens it read and wrote, of which the tokens it read again and the tokens it kept to read again, and its dollars.
A usage holds the token counts and the dollars of one model response.
The share of the window it filled is the tokens it read against the window of the actor its rung names, in the standing where the answer lands, so no word of it says the share.
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
"""An actor the World offers: the name of a model, the efforts it takes and the window it reads.
The window that a roster entry leaves unsaid is the window that the file names.
What a prompt names is one of these names and one effort of that range.
An actor takes an effort of its own, and any actor takes the effort that is not named.
"""
type Standing = list[list[Actor] | str]
"""What a chain stands on: the actors the World offers, the directory the chain starts in, the actor a prompt goes to when it names none, and the path of the doctrine, which the World reads in the first turn of the setup of the root, and which a World that gives no doctrine leaves unsaid.
The roster, the directory and the actor that a model reads are in the transcript of its chain.
A standing holds no source: the engine is one file the model imports, and a record replays only on an engine that makes each of its acts again, and else drifts.
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
An act that is over says nothing, and the chain answers a read of the doors of a command that is over.
A done that an act said itself is the result of the act.
A done that an ear says while the act is put to it is the answer to the act, which takes it now.
A kind that ends when it is told to: it starts its ear, and then a done that names it is what it came to; a cancel over it ends it with a CancelledError, and a close of it with the value that close carries.
"""
type Tell = tuple[Literal["tell"], str, str, list[Note]]
"""A tell carries notes about the act it is about, and the turns are folded from them.
What an act tells is the telling of its chain, so the journal keeps none of it, and a later life tells it again.
A tell is the one fact that carries notes, which the turns are folded from: a control tells its header by a tell.
"""
type Pause = tuple[Literal["pause"], str, str, object]
"""While a pause stands, nothing that the pause is over hears, and what is said meanwhile waits for the wake.
A control is a fact over an act: over that act, over everything that act made, and over everything on a chain when the control names a chain.
A control reaches what it is over, and whatever else the words of the control name.
A control is no act: it takes no name of its own, and the record holds it as a fact about the acts it is over.
What a control reaches: the act it names, everything that act made, and every act of the chain it names.
It reaches by the chain as well as by the name, since an act on a chain is not under it unless the chain made it.
A control from outside a chain tells its header by a tell that its call says just before it, so a model reads what another did to its work, and nothing else writes that header, so a pause of a grant at its ceiling and a close of a prompt that its chain will not serve, which come from inside it, tell none.
A pause is over the act it names and everything under it.
"""
type Wake = tuple[Literal["wake"], str, str]
"""A wake ends the pause over the same act, and what waited is heard.
A wake is over the act it names and everything under it.
"""
type Cancel = tuple[Literal["cancel"], str, str, object]
"""A cancel ends everything it is over: each of them is done with CancelledError, and none of them says anything of its own again.
A cancel reaches to any depth, and on whatever chain.
"""
type Close = tuple[Literal["close"], str, str, object]
"""A close is a cancel that carries what the act it names is done with, and it is a kind of its own, since the word of a cancel is its cause, which the act raises as a CancelledError, where the word of a close is what the act comes to.
A close is over the act it names, the words running under it and the replies that ask for those words, where a cancel is over everything under it.
"""
type Started = tuple[Literal["started"], str, str]
"""What an ear says to take an act whose done comes later, and which no ear after it hears, a chain apart.
The started names the act and says no more of it, since the ear heard the act itself and nothing is told twice.
The journal keeps a started of the outside, so a later life holds the act from the outside and does not say its first started again, since only the outside runs it: the act is done where the record holds its done, and pending when the record holds none.
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
The life refuses a reply that no ear owns, and the chain that asks then pauses itself with that refusal as its cause, since that fault stands until a wake.
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
The chain asks no model while a word of the chain runs, so every word that is ready runs before a model is asked.
The chain asks one model at a time, which it reads from its transcript, the rung that has waited the longest among those it heard on itself that nothing has been said of, handing it the turns as they stand, for the World to hand its provider as it likes, so that many chains ask many models at once.
"""
type Ready = tuple[Literal["ready"], str, str, str]
"""A ready says the word a rung holds.
The word of a rung enters the program of the chain when the gate accepts it or when the chain wrote it, and runs in the globals of the chain.
The word of a rung that extends the engine is part of the program, so the extension returns in a later life.
The old words stay in the program and in the turns after a rung rebinds a name.
"""
type Run = tuple[Literal["run"], str, str, str, str, str, str | None]
"""A run is the act of running the word of a rung or the text of a told rung, which the chain makes on itself and the Kernel takes: it says started as the run, runs what the writings of that rung left of the word, as the rung, and says the run done with what the word gave.
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
A run whose rung is done when the Kernel begins it never begins its word, and is done with CancelledError.
"""
type Writing = tuple[Literal["writing"], str, str, str, int]
"""A writing is the fact that holds a part of the word of a rung, which the World says while the model of the rung writes the word, and which the Kernel hears and runs.
A writing holds the part of the word that its model finished since the writing before it, and how many characters of the word stand before that part.
The Kernel runs the part of a writing in the globals of the chain of its rung and as that rung, so what the part binds and what it makes are the rung's own.
The Kernel runs the writings of one rung in the order it heard them, and a part that waits for an act holds the parts after it.
A part that waits for an act while its model writes makes no wants, so its rung hears the turn of its model while the part waits.
Once the run of its rung begins, a part that waits for an act waits with a wants as that run, as a run does, so the chain asks its next prompt while the word waits.
The run of a rung runs what the writings of that rung left of the word, and nothing when they ran the word whole.
The run of a rung whose writing raised is done with that raise and runs no more of the word, since a word is over where it raises.
A run whose word is not the word that ran is refused, since the Kernel runs no statement of a word twice.
"""
type Ran = tuple[Literal["ran"], str, str, int, BaseException | None]
"""A ran is the fact that the Kernel says as a rung once the part of a writing of that rung is over, with how many characters of the word stand before that part, and with what a statement of the part raised, or nothing when the part ran to its end.
The ran of a writing whose part follows another word than the word that ran holds a refusal, and its part never begins, since the Kernel runs no statement of a word twice.
The ran of a writing of a rung that is done holds CancelledError, since its part never begins.
"""
type Wants = tuple[Literal["wants"], str, str, str, str]
"""A wants is the act that a run makes when its word waits for an act that is not done: the rung takes it, and answers it with what that act came to when the rung hears its done, so a pause over the rung holds the word."""
type Out = tuple[Literal["out"], str, str, str, str]
"""The streams of a command come as out facts while the command runs, which the journal keeps.
A later life reads the parts that a command told before the death of the process.
"""
type Read = tuple[Literal["read"], str, str, str, str]
"""A read is the question of the text at a path."""
type Write = tuple[Literal["write"], str, str, str, Text]
"""A write is the question of putting a text at its path."""
type Clock = tuple[Literal["clock"], str, str, str]
"""A clock is the question of a reading of the wall clock."""
type Chance = tuple[Literal["chance"], str, str, str]
"""A chance is the question of a number the World draws."""
type Stand = tuple[Literal["stand"], str, str, str]
"""A stand is the question of what the chains stand on, which boot asks the World on the root at the tip of every life, and whose answer binds the actor of every chain from its place on.
A change of the World between two lives enters the transcript of a chain.
The World answers a stand with the roster, the directory, the actor and the path of the doctrine.
A model asked on any chain of a later life finds the new roster in the transcript of its chain, in the stand that its chain plays there.
The World answers it with a done at once, so the life stands on that answer before boot returns.
The journal keeps each stand and its answer, so a later life says them again at their places and replays every chain on what it stood on there.
At its tip, once the record is said again whole, boot stands the life again, so a change of the World reaches every chain after what it replayed.
Every chain hears the done of every stand, and binds its default actor again when the actor of the standing changes, and a chain whose standing names a doctrine plays on itself the rung of stand(), with a cd to the new directory when the directory changed and no cd of a rung with no actor moved the chain, so the transcript grows at one end.
A standing binds the default actor of the chain, under the name actor, when its actor differs from that of the standing before it, so a new roster or a new directory alone keeps the actor that a word chose.
"""
type Module = tuple[Literal["module"], str, str, dict[str, object]]
"""A module carries the globals of a chain, which the chain says at its birth and at each replay, so its transcript says which module each of its rungs ran in."""
type Prefix = tuple[Literal["prefix"], str, str, list[Fact]]
"""A prefix carries what a chain with a source holds of the transcript of its origin, which the chain says at its birth, before its open.
The transcript of the chain holds the facts of its prefix where the prefix stands, and not the prefix itself.
A prefix takes the place of what the transcript of the chain that it is said on held, so a word that says a prefix on its own chain cuts its turns, and the chain goes on under its name, with its module, its ladders and its acts.
"""
type Gate = tuple[Literal["gate"], str, str, str, str]
"""A gate is the question of whether a word may run, which the ear named gate answers with its findings, apart from the Kernel, so a word may ask it while the Kernel runs that word.
A gate carries the word alone, and the gate reads the program of the chain before that rung and the names of its acts when it takes the gate, and the word after it, so the Kernel keeps no ladder of its own, the journal keeps no program, and a word of a program made again is read after the rungs that stand.
The refused paragraph binds the findings that refused the word of a rung, one line for each, as rungN_findings.
The chain has the word of a rung gated before it runs, but a word it wrote itself, and a refused word runs never.
A rung that retells stands with the gate where the one it retells stood, so the gate reads a word once in a life, and a replay makes no copy of a refused word, which never ran, so the chain tells nothing of that word again.
A refused word of a rung stands in the ladder of its prompt, which its door shows, and it is no part of the program of the chain, which holds the words that run.
The chain tells the findings that refused a word, ends that rung with a refusal that holds them, and the prompt of it asks again as it does for a word that gave no value.
A word that the chain cannot gate or run, as when the life refuses its gate, is refused as the gate refuses a word, with what went wrong as its one finding, so no word and no told text ends the ear of a chain.
"""
type Cd = tuple[Literal["cd"], str, str, str, str]
"""A cd is the question of the directory a path names, which the World answers and the transcript of the chain holds."""
type Merged = tuple[Literal["merged"], str, str, str, str]
"""A merged is the question of whether the stderr of a command flows into its stdout, which the command answers from what its verb was given, and which the World asks when it takes the command.
The World runs a merged command with its stderr in its stdout, so no out fact of it is of its stderr, and the door of its stderr stays empty.
"""
type Wait = tuple[Literal["wait"], str, str, str, float, *tuple[str, ...]]
"""A wait carries the seconds that must pass before the World is done with it, and the names of its acts, which are plain, so the journal makes it again."""
type Rung = tuple[Literal["rung"], str, str, str, str, str, str]
"""A rung carries the word, the rung it retells and the actor."""
type Prompt = tuple[Literal["prompt"], str, str, str, str, str, str]
"""A prompt carries the name of the shape, the message and the actor of a prompt."""
type Chain = tuple[Literal["chain"], str, str, str, str, str]
"""A chain carries the label and the source of a chain, and every chain is chainN whether boot or chain opened it."""
type Grant = tuple[Literal["grant"], str, str, str, float | None, float | None]
"""A grant carries the ceiling in dollars and the ceiling in share of the window."""
type Bash = tuple[Literal["bash"], str, str, str, str, str | None, float]
"""A bash carries the command, the stdin and the timeout, and no show and no working directory."""

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

def prompted(name: str) -> bool:
  """Whether a name is the name of a prompt, which its name says, as prompt3, and not prompt, chain3 or an empty name."""

def scope(name: str) -> str:
  """The scope of a question, from its name: the chain it is on, or the scope of the thread it is on, and itself for a chain, and nothing for a name of no question of the life."""

def answer[T](kind: str, on: str, empty: T) -> T:
  """The answer of the last question of a kind on a chain that is of the type of empty, and empty when no such question stands."""

def filled(about: str) -> float:
  """The share of the window that a reply read, to two places: the tokens that it read over the window that the roster of the standing gives its actor, and over WINDOW when the roster holds no such actor, and nothing when the reply gave no turn that holds a usage."""

def over(id: str) -> bool:
  """Whether an act is over: its done stands, so the life holds its outcome under its name, and a name of no act is never over."""

def tell(on: str, path: str = "", got: object = None, show: Show = HEAD, **words: object) -> None:
  """What a question answered now that shows a text or changes a state tells of itself: a paragraph headed with the name of the question, which binds its path and what it was answered, said by the run that asked it about that question, and nothing at all outside a run; one that only reads a value tells nothing, since the word that asked it holds the value, which it debugs to see.
  A question is put to the ears of the engine before those of the outside, so the World is asked for nothing that the engine knows.
  The question that tell names is the last question on its chain that the run made.
  A text is told by the lines that its show picks, under the word text, and any other answer under the word value.
  When the lines that a show picks of a text are not one run, the tell binds their numbers too, under the word numbers, before the text, so a model can place each line.
  """

def tells(id: str) -> bool:
  """Whether an act tells: a rung that its chain made to retell another tells nothing, neither its word nor what its word does, since what it would tell stands told already.
  The rung of the stand that a chain plays on itself retells no rung, so it tells as any rung, and the stand and the cd of its word tell their paragraphs.
  """

def crafted(id: str) -> list[str]:
  """The rungs with a word and an actor that are open on the chain of an act, by their names, in the order that they were made, which a rung with a word, a prompt to a model and the open of the root wait for."""

def told(id: str, what: str = "", *notes: Note, **words: object) -> Tell:
  """The open of an act tells the id and what the act says of itself, and no actor and no arguments as such.
  A closed paragraph binds what the act came to under the word value.
  told gives the saying of a tell about an act, with one paragraph headed with the id of the act, which an ear yields and a verb says, so a chain holds what it told where it told it.
  """

def control(kind: str, name: str, id: str, *words: object) -> Fact:
  """A control said over the act it names, after a tell of a header of its name headed with that act when it comes from outside the chain of that act, which is how pause, wake, cancel and close say theirs.
  The call gives the control as the life made it whole, so a close that named no act reads which act it is over.
  A control is said over any act of the life, and it tells its header only while the act it names is not over, so a control over an act that is over tells nothing and still reaches each act under it that is not over.
  The header of a control names the one that said it, as #rung5 paused by operator, or #prompt4 closed by rung9 when rung9 stands on another chain, so a turn says who paused, woke, cancelled or closed the act.
  A control from inside the chain of the act it names, as one that a word or an ear of that chain says, tells nothing, since the chain shows already what its words and its ears did.
  """

def headed(id: str, what: str = "", **words: object) -> str:
  """The paragraph of an act: its header, # and the id with no space between, then what happened, and under it the binding of each word, in order.
  headed binds each value by its repr with no check, since every value that reaches it is a wire value, and the repr of a wire value is python.
  headed shows a value as python shows it, less the path of the module before the name of its class, since a chain binds a class by its name alone.
  """

def opened(id: str, of: str, **words: object) -> Tell:
  """The paragraph that opens an act: its header with no event and the binding of each of its words, then the statement that binds the name of the act to the act, with the type of what the act comes to, as bash1: Act[Exit] = Act('bash1'), so the gate knows what an await of it gives."""

def shown(text: Text, show: Show) -> str:
  """The lines of a text that a show picks, as they are, one on each line, each cut at its first 1000 characters and then marked with its length, as ... (1045 chars), so a long line of a command or of a file does not fill the turns and a cut line does not read as the end of the text."""

def unquoted(word: str) -> str:
  """What a word is as python: each quote in it bound as a string.
  A quote is a string a word writes between two marks, <s:name> at the start of a line and </s:name> at the end of a line, name being any python name, so nothing in it needs an escape.
  The open mark is looked for line by line from the top, its close mark from the open mark down, and the next open mark after that close mark, so the text of a quote holds the marks of any quote as text.
  The value of a quote is the text between its marks, less a line break just after the open mark.
  A quote becomes its name bound to its value as python writes it, on the line of the open mark, and each other line of the quote becomes an empty line, so every line after it keeps its number.
  An open mark that has no close mark stays as it is, and the gate reads it as the python it is not.
  A word unquotes in a time that grows with its length alone, so a chain that was told a text of any length goes on.
  The engine unquotes a word before the gate reads it and before the Kernel runs it, and the door of a ladder and the turns keep the quotes as the word wrote them.
  """

def offered(roster: list[Actor], to: str) -> int | None:
  """The window of the roster entry of an actor, and nothing at all when the roster holds no such actor, or when that one takes no such effort."""

def covers(a: Fact, id: str) -> bool:
  """Whether a control is over an act: over the act it names and everything under it, and over every act on the chain it names; a close is over the act it names, each rung that act made and each reply that asks for the word of one of those, where a cancel is over everything under it.
  A rung that a word makes is an act of that word, as a command is, so a close of the prompt of that word is not over it, on whatever chain it runs.
  A control over a thread is over each prompt on it too, and over what a control over that prompt is over.
  """

def paused(id: str) -> bool:
  """paused reads whether an act is paused off the transcript of its chain, so an ear born while a pause over its act stands is born paused, and takes its act all the same."""

def idle(id: str) -> Generator[set[str], Fact]:
  """idle waits for the facts about its own name and says nothing of its own, which is the ear of a reply and the ear of the operator."""

def lives(g: Ear, a: Fact | None) -> set[str] | Literal[False] | None:
  """lives carries a fact into an ear, says everything the ear yields, and gives what the ear waits for, or False once the ear is over."""

def controlled(ear: Callable[[str], Ear], lasting: bool = False) -> Callable[[str], Ear]:
  """An act is paused while the last control in record order that is over it is a pause.
  A pause holds delivery: a result that arrives enters the record and waits.
  A paused prompt stops at its next boundary, with its loop where it stood.
  The engine holds the response of a reply that returns on a paused chain.
  A rung carries on only while its own chain is not paused.
  A control from outside reaches a paused act at once, where what the words of the act say waits for the wake.
  A paused ear hears at once an act put to it, and every other fact at the wake.
  A close from outside still ends what a pause is over.
  What an act a control is over is done with: the value a close carries for the act it names, and a CancelledError for every other.
  An ear that is not lasting is over as it says a done of its own, so it never hears that done and says nothing after it.
  A controlled ear waits for what the ear that it carries waits for.
  A lasting ear lives past the done of its act until it returns, as the ear of a command does to hear that done, whoever said it, and to tell its exit, and a cancel or a close over its act ends the act all the same.
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
  A fact the journal says again is the journal's own, as is an answer it says again when an act is made again, but a refusal of the life, which it says again under no site.
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
  The journal keeps a joined mark, ("joined", "", "journal", kind, act), which names the kind and the act of the fact that it joins, before a fact that it keeps right after the fact it kept last, with no other fact heard between them, since one ear said both while it heard one fact, and a later life says that fact again at once after the one before it, so the ears hear that burst in one burst again, as the first life did.
  A fact that the journal keeps after a fact that it does not keep holds no mark, so a later life says it again only once every fact said before it has been heard, after the reaction that came between them, as the first life did.
  The journal says the whole record again before boot returns, so no entry waits for an act that a host makes after boot.
  One said again takes the name it had, since a later life makes its acts again in the order of the record, and the life counts each kind on from there, so that no later act takes a name that is taken.
  A boot is a life; a second boot is a second life, and the first is gone.
  The names operator and journal are the life's own, and boot refuses a generator of the outside under either name.
  A life settles an await of its acts from outside a run in the loop it is opened in, so boot outside a running loop raises before it makes anything.
  An act whose maker is neither an act nor an ear that boot was given is said again through its verb, with the words the record holds and the chain it names, so its ear is the verb's, a host finds it made when boot returns and makes it not again, and a show or a filter it was given is not said again, since the record holds none.
  A later life says an act whose maker is neither an act nor an ear that boot was given again only through a verb in the globals of its chain, and an act whose kind no verb binds is a drift.
  The journal says again a done that is the first answer of an act when the act is made again, and every later fact of it at the place where the record holds it, the started of each act that a wake began again among them.
  A first answer that the record holds with no maker is a refusal of the life, that nothing took the act, so the journal says it again under no site, as the life said it, and neither keeps it again nor reads it as a drift.
  What the record shows started and not done when boot returns is pending: the journal holds it from the outside until a wake that this life says.
  """

actor: str
"""actor is the default actor of the chain, bound from the standing, and empty in the module of the engine, where no chain stands.
The program rebinds actor like any name, and the last binding wins.
"""
raised: BaseException | None
"""raised is the exception object that the last rung raised, rebound at each raise but a CancelledError.
raised is None at the birth of the module of a chain, so a word reads it before any rung raised."""
