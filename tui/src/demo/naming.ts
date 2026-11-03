import type { Script } from "./script.ts";

/** What the model of the demo answers when the extension naming asks it to name a session: the first line of the
 * first message, which the brief of the extension ends with. It stands before every other script, since the brief holds
 * the message, whose cue another script would take. */
export const naming: Script[] = [
  {
    cue: "Give a name to a session of work",
    steps: [
      {
        thinking: "The first line of the message says what the work is.",
        word: [
          'asked = [x for x in transcript() if x[0] == "prompt"][-1][5]',
          'close(asked.split("The first message:\\n")[-1].strip().splitlines()[0][:40].rstrip(" ."))',
        ].join("\n"),
      },
    ],
  },
];
