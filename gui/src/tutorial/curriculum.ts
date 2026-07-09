// The built-in lesson plan: a sequence of guided tutorials from "what even is
// this app" to building real models, each targeting real on-screen elements
// (via `target`, a CSS selector matching a `data-tutorial="..."` attribute
// placed on the real component) so the highlight is never fake UI drawn over
// a screenshot — it's the actual app.
//
// `focusTab` switches the side/bottom tab rail to the right tab before a step
// is shown (see Tabs.tsx's `forceActive` prop), so a step can point at
// something inside a tab that isn't currently open.

export type Difficulty = 'beginner' | 'intermediate' | 'advanced';

export interface TutorialStep {
  title: string;
  body: string;
  /** CSS selector for the element to spotlight. Omit for a centered, no-target step (e.g. an intro/outro). */
  target?: string;
  focusTab?: { slot: 'side' | 'bottom'; tabId: string };
}

export interface Tutorial {
  id: string;
  title: string;
  blurb: string;
  difficulty: Difficulty;
  minutes: number;
  steps: TutorialStep[];
}

export const CURRICULUM: Tutorial[] = [
  {
    id: 'orientation',
    title: 'Welcome to BrainBuilder',
    blurb: "A 2-minute tour of the screen before you build anything.",
    difficulty: 'beginner',
    minutes: 2,
    steps: [
      {
        title: 'Welcome!',
        body:
          "BrainBuilder helps you build your own AI — a computer program that learns from examples, instead " +
          "of being told exact rules. Let's take a quick look around before you build your first one.",
      },
      {
        title: 'The Components shelf',
        body:
          'This is your toolbox. Each block here is a building piece for your AI — like LEGO bricks, but for ' +
          "thinking machines. You won't need to touch this yet — we'll start somewhere much easier.",
        target: '[data-tutorial="palette"]',
      },
      {
        title: 'The canvas',
        body:
          "This big empty space is where your AI gets built, piece by piece. Right now it's empty — that's " +
          'about to change.',
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'The Build tab',
        body:
          'This is where the magic starts. Instead of building piece by piece, you can just describe what ' +
          'you want, point at some examples, and BrainBuilder builds it for you.',
        target: '[data-tutorial="tab-build"]',
        focusTab: { slot: 'side', tabId: 'build' },
      },
      {
        title: 'Metrics and Predict',
        body:
          "Down here you'll watch your AI learn (Metrics), and later try it out on new things it's never " +
          'seen (Predict).',
        target: '[data-tutorial="tab-metrics"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
      {
        title: "You're ready!",
        body: 'That\'s the whole screen. Next up: "Build Your First Image Classifier" — a real, working AI, start to finish.',
      },
    ],
  },
  {
    id: 'first-classifier',
    title: 'Build Your First Image Classifier',
    blurb: 'Teach a real AI to sort pictures into categories you pick — start to finish, no experience needed.',
    difficulty: 'beginner',
    minutes: 8,
    steps: [
      {
        title: "Let's build something that sees",
        body:
          'You\'re going to build an AI that looks at pictures and sorts them into groups — like "cat" vs ' +
          '"dog", or any two (or more) things you have pictures of. All you need is a folder of pictures, ' +
          "organized into subfolders by what's in them.",
        target: '[data-tutorial="tab-build"]',
        focusTab: { slot: 'side', tabId: 'build' },
      },
      {
        title: '1. Say what you want',
        body:
          'This dropdown says "Sort my data into categories" — that\'s exactly what we want. Leave it as is.',
        target: '[data-tutorial="intent-task"]',
      },
      {
        title: '2. Point at your pictures',
        body:
          'Choose "A folder of images". Then click "Choose folder" and pick a folder with subfolders inside ' +
          "it — one subfolder per category (e.g. a \"cats\" folder and a \"dogs\" folder, each full of " +
          'pictures). BrainBuilder figures out the categories from your folder names.',
        target: '[data-tutorial="intent-data"]',
      },
      {
        title: '3. Build it',
        body:
          'Click "Build my model". BrainBuilder looks at your pictures, decides how big the AI needs to be, ' +
          'and builds it for you automatically — no wiring, no settings to tune.',
        target: '[data-tutorial="intent-build-btn"]',
      },
      {
        title: 'Watch it learn',
        body:
          'Switch to the Metrics tab and click "Export & Train" on the canvas toolbar. Watch the loss number ' +
          '— that\'s how wrong the AI currently is. Watching it go down means your AI is getting smarter in ' +
          'real time.',
        target: '[data-tutorial="tab-metrics"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
      {
        title: 'Try it out',
        body:
          'Once training finishes, switch to Predict and show your AI a new picture it has never seen. See ' +
          'if it gets it right!',
        target: '[data-tutorial="tab-predict"]',
        focusTab: { slot: 'bottom', tabId: 'predict' },
      },
      {
        title: 'You built a real AI!',
        body:
          "That's it — you just built and trained a working neural network. Try \"Look Inside Your Model\" " +
          'next to see what BrainBuilder actually built for you.',
      },
    ],
  },
  {
    id: 'first-text-classifier',
    title: 'Build a Text Classifier',
    blurb: 'Teach an AI to sort sentences or reviews into categories — same idea as pictures, different data.',
    difficulty: 'beginner',
    minutes: 8,
    steps: [
      {
        title: "Let's build something that reads",
        body:
          'This time your AI will read text instead of looking at pictures — like sorting reviews into ' +
          '"positive" vs "negative", or messages into "spam" vs "not spam". You need a spreadsheet with one ' +
          'column of text and (ideally) one column of labels.',
        target: '[data-tutorial="tab-build"]',
        focusTab: { slot: 'side', tabId: 'build' },
      },
      {
        title: '1. Say what you want',
        body: 'Keep "Sort my data into categories" selected — sorting text is still classification.',
        target: '[data-tutorial="intent-task"]',
      },
      {
        title: '2. Point at your spreadsheet',
        body:
          'Choose "A spreadsheet with a text column". Click "Choose file" and pick your CSV or Parquet file. ' +
          'Then type the exact name of the column that holds the text (e.g. "review" or "message") — this one ' +
          "is required, unlike the picture folder step, because a spreadsheet has many columns and BrainBuilder " +
          "can't guess which one is the text.",
        target: '[data-tutorial="intent-data"]',
      },
      {
        title: '(Optional) Point at your labels',
        body:
          "If your spreadsheet has a column of correct answers (e.g. \"positive\"/\"negative\"), type its name " +
          'in the second box. Leave it blank and BrainBuilder assumes the last column is the label.',
        target: '[data-tutorial="intent-data"]',
      },
      {
        title: '3. Build it',
        body: 'Click "Build my model". BrainBuilder turns your words into numbers and sizes a model to fit.',
        target: '[data-tutorial="intent-build-btn"]',
      },
      {
        title: 'Watch it learn, then try it',
        body:
          'Switch to Metrics, click "Export & Train", and watch the loss go down. Once it finishes, switch to ' +
          'Predict and type a new sentence to see what category your AI thinks it belongs to.',
        target: '[data-tutorial="tab-metrics"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
      {
        title: "You taught an AI to read!",
        body:
          "Same building blocks, same steps, completely different kind of data — that's the pattern for " +
          'almost everything in BrainBuilder. Try "Predict a Number" next for a third kind of task.',
      },
    ],
  },
  {
    id: 'first-regression',
    title: 'Predict a Number',
    blurb: 'Teach an AI to guess a number — a price, a score, a temperature — instead of a category.',
    difficulty: 'beginner',
    minutes: 8,
    steps: [
      {
        title: "Not every question has a category answer",
        body:
          '"Is this a cat or a dog" has a category answer. "How much should this house sell for" has a number ' +
          'answer instead. This is called regression, and it uses the exact same Build tab — just a different ' +
          'setting.',
        target: '[data-tutorial="tab-build"]',
        focusTab: { slot: 'side', tabId: 'build' },
      },
      {
        title: '1. Say what you want',
        body: 'Change the dropdown to "Predict a number" — this tells BrainBuilder you want a number back, not a category name.',
        target: '[data-tutorial="intent-task"]',
      },
      {
        title: '2. Point at your spreadsheet',
        body:
          'Choose "A spreadsheet of numbers (CSV/Parquet)" and pick your file. If you want to predict a ' +
          'specific column, type its name below — otherwise BrainBuilder predicts the last column by default.',
        target: '[data-tutorial="intent-data"]',
      },
      {
        title: '3. Build it',
        body: 'Click "Build my model". BrainBuilder shapes a model whose final answer is one number, not a list of categories.',
        target: '[data-tutorial="intent-build-btn"]',
      },
      {
        title: 'Watch it learn',
        body:
          'Switch to Metrics and train. The loss number here means the same thing as before: how far off the ' +
          "guessed numbers are from the real ones, on average. Smaller is better.",
        target: '[data-tutorial="tab-metrics"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
      {
        title: 'Try a prediction',
        body: 'Switch to Predict and feed it a new row of numbers to see what it guesses.',
        target: '[data-tutorial="tab-predict"]',
        focusTab: { slot: 'bottom', tabId: 'predict' },
      },
      {
        title: 'Three task types down',
        body:
          "Pictures, text, and numbers — you now know the on-ramp for all three kinds of tasks BrainBuilder " +
          'supports. Everything past this point ("Look Inside Your Model" onward) is about understanding and ' +
          'customizing what gets built, not new kinds of data.',
      },
    ],
  },
  {
    id: 'look-inside',
    title: 'Look Inside Your Model',
    blurb: 'See the actual building blocks BrainBuilder assembled for you, and what each one does.',
    difficulty: 'intermediate',
    minutes: 5,
    steps: [
      {
        title: 'Peek under the hood',
        body:
          "After building a model in the Build tab, the canvas isn't empty anymore — it's full of connected " +
          'boxes. Each box is a "layer": one step in how your AI thinks about a picture. This tutorial works ' +
          "best if you've already built the image classifier from the last tutorial.",
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'Every box has a job',
        body:
          'Some boxes look for simple patterns (edges, colors). Others combine those patterns into shapes, ' +
          'and later boxes combine shapes into whole objects — like a cat\'s ear or a dog\'s nose. The last ' +
          'box turns all of that into a final decision.',
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'Click a box',
        body:
          'Click on any node on the canvas. The Inspector tab (in the side rail) will show you its settings — ' +
          "these are called hyperparameters: numbers that control how that box behaves. Don't worry about " +
          'changing them yet, just look.',
        target: '[data-tutorial="tab-inspector"]',
        focusTab: { slot: 'side', tabId: 'inspector' },
      },
      {
        title: 'Numbers next to each box',
        body:
          'You may notice small numbers near each box — that\'s the shape: how many values are flowing ' +
          "through at that point. BrainBuilder checks these automatically so two boxes that don't fit " +
          "together get flagged before you waste time training.",
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'Data flows left to right',
        body:
          'Follow the lines between boxes — that\'s the path your picture takes, transformed step by step, ' +
          'until the last box gives the final answer (which category it thinks the picture belongs to).',
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'Watch it train, again, slower this time',
        body:
          'Switch to the Metrics tab and re-run training. This time, picture each loss update as one of the ' +
          'boxes you just looked at nudging its numbers very slightly toward being less wrong. Millions of ' +
          "tiny nudges add up to a model that works.",
        target: '[data-tutorial="tab-metrics"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
      {
        title: 'More boxes to explore',
        body:
          "Open the Components shelf to see every kind of building block BrainBuilder knows about. You'll " +
          "meet these again in \"Build From a Template\", where you'll assemble one yourself.",
        target: '[data-tutorial="palette"]',
      },
    ],
  },
  {
    id: 'look-inside-text',
    title: 'Look Inside a Text Model',
    blurb: 'See how BrainBuilder turns words into numbers, and why that matters.',
    difficulty: 'intermediate',
    minutes: 5,
    steps: [
      {
        title: 'Words become numbers first',
        body:
          "Build the text classifier from the last tutorial if you haven't already, then look at the canvas. " +
          "The very first box a text model uses is an embedding — it turns each word into a list of numbers. " +
          "Nothing downstream understands letters, only numbers.",
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'Similar words end up close together',
        body:
          'As training goes on, the embedding box learns to give similar words similar numbers — "great" and ' +
          '"excellent" drift toward each other, while "great" and "terrible" drift apart. Nobody tells it ' +
          'this directly; it discovers it from seeing lots of examples.',
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'One sentence, many words, one answer',
        body:
          'A sentence is a whole sequence of word-numbers, but your model needs to output just one answer ' +
          '(e.g. "positive"). Somewhere in the middle, the boxes combine every word\'s numbers into a single ' +
          "summary before the last box makes a decision from it.",
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'Click a box to see its settings',
        body:
          'Click the embedding box and open the Inspector. You\'ll see a setting like "vocab size" — the ' +
          "number of different words the model can recognize at all. A word it's never seen before can't be " +
          'looked up, which is why more training text usually helps.',
        target: '[data-tutorial="tab-inspector"]',
        focusTab: { slot: 'side', tabId: 'inspector' },
      },
      {
        title: 'Same rules as pictures, different data',
        body:
          "Everything you learned in \"Look Inside Your Model\" still applies — shapes must match, data flows " +
          'left to right, training is nudging numbers to be less wrong. Only the very first step (turning the ' +
          'raw input into numbers) looks different between pictures and text.',
        target: '[data-tutorial="canvas"]',
      },
    ],
  },
  {
    id: 'understanding-regression',
    title: 'Why Regression Looks Different',
    blurb: "See how a number-predicting model's last box and loss differ from a category-sorting one.",
    difficulty: 'intermediate',
    minutes: 4,
    steps: [
      {
        title: 'No categories to pick from',
        body:
          "Build the number-predicting model from \"Predict a Number\" if you haven't already. Click its last " +
          'box on the canvas and open the Inspector — notice it outputs just one number, not a list of ' +
          'category scores like the image or text classifiers did.',
        target: '[data-tutorial="tab-inspector"]',
        focusTab: { slot: 'side', tabId: 'inspector' },
      },
      {
        title: '"Close" is good, not just "right"',
        body:
          'A classifier is either right or wrong about a category. A regression model can be a little wrong or ' +
          'a lot wrong — guessing $205,000 for a $200,000 house is much better than guessing $50,000, even ' +
          "though neither is exactly right. Loss captures that difference in distance, not just yes/no.",
        target: '[data-tutorial="tab-metrics"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
      {
        title: 'Watch the loss number itself',
        body:
          "For classification, loss is a somewhat abstract score. For regression, it's often close to the " +
          "actual size of your average mistake — if loss is around 400, your model's guesses are typically " +
          'off by somewhere around that many units (dollars, degrees, whatever you\'re predicting).',
        target: '[data-tutorial="tab-metrics"]',
      },
      {
        title: 'Same training, different finish line',
        body:
          'Everything else — layers, activations, training loop, watching loss drop — works exactly like ' +
          'classification. Only the very last box and how "correct" gets measured are different.',
      },
    ],
  },
  {
    id: 'template-build',
    title: 'Build From a Template',
    blurb: 'Start from a ready-made blueprint, see how the pieces connect, and change one on purpose.',
    difficulty: 'intermediate',
    minutes: 6,
    steps: [
      {
        title: 'Templates: pre-made blueprints',
        body:
          'A template is a ready-made arrangement of boxes, already connected correctly. It\'s a great way to ' +
          'see a working design without building it from an empty canvas — like a recipe instead of guessing ' +
          'ingredients.',
        target: '[data-tutorial="tab-templates"]',
        focusTab: { slot: 'side', tabId: 'templates' },
      },
      {
        title: 'Pick one and click Use',
        body:
          'Try "MLP Classifier" — a simple, classic design (MLP is short for "multi-layer perceptron", one of ' +
          'the oldest and most reliable neural network shapes). Click "Use" and watch the canvas fill in with ' +
          'connected boxes instantly.',
        target: '[data-tutorial="tab-templates"]',
      },
      {
        title: 'Compare it to your first model',
        body:
          'Notice this one looks different from the image classifier you built earlier — different tasks ' +
          'need different shaped AIs. BrainBuilder (and templates) pick the right shape for the job.',
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'Change one thing on purpose',
        body:
          'Click one of the boxes and open the Inspector. Try changing a number — like making a layer bigger. ' +
          'Bigger usually means the AI can learn more complex patterns, but also takes longer to train and ' +
          'needs more examples to avoid just memorizing them.',
        target: '[data-tutorial="tab-inspector"]',
        focusTab: { slot: 'side', tabId: 'inspector' },
      },
      {
        title: 'Check your change is still valid',
        body:
          'Click "Validate" on the canvas toolbar. If your change broke how two boxes fit together, ' +
          "BrainBuilder tells you immediately instead of letting you train something broken.",
        target: '[data-tutorial="validate-btn"]',
      },
      {
        title: 'Train your modified template',
        body:
          'Head to Metrics and train it. Compare the results to your original — this is exactly how real ' +
          'AI researchers experiment: change one thing, retrain, compare.',
        target: '[data-tutorial="tab-metrics"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
    ],
  },
  {
    id: 'auto-tune',
    title: 'Let BrainBuilder Pick Your Settings',
    blurb: 'Skip the guesswork — auto-tune runs a few quick trials and applies whichever settings trained best.',
    difficulty: 'intermediate',
    minutes: 5,
    steps: [
      {
        title: "Don't want to guess the settings?",
        body:
          "The Data tab has settings like learning rate, batch size, and optimizer — hyperparameters you'd " +
          "otherwise have to guess at. Auto-tune tries several combinations for you and keeps whichever one " +
          'actually trained best, on your real model and real data.',
        target: '[data-tutorial="tab-data"]',
        focusTab: { slot: 'side', tabId: 'data' },
      },
      {
        title: "It's real training, just short",
        body:
          'Behind the "Auto-tune" button, BrainBuilder runs several short training trials, each with a ' +
          "different learning rate, batch size, or optimizer. Every trial is genuinely trained, just for a " +
          "few steps instead of a full run — enough to tell which setup is learning faster.",
        target: '[data-tutorial="autotune-btn"]',
      },
      {
        title: '(Optional) Also search the shape',
        body:
          'Check "also try narrower / wider models" to let auto-tune try slightly smaller and bigger versions ' +
          "of your model too, not just the training settings — useful if you're not sure whether your model " +
          'is the right size for your data.',
        target: '[data-tutorial="autotune-search-arch"]',
      },
      {
        title: 'Run it and read the results',
        body:
          'Click "Auto-tune" and wait for the trials to finish. You\'ll get a ranked list — the winner\'s ' +
          'learning rate, batch size, and optimizer are applied automatically, ready for you to just click ' +
          '"Export & Train" with.',
        target: '[data-tutorial="autotune-results"]',
      },
      {
        title: 'Train with the picked settings',
        body:
          "Switch to Metrics and train as usual. You're using real, tested settings instead of a guess — " +
          "and now you know what \"learning rate\" and \"batch size\" actually do: they're exactly what " +
          'auto-tune just experimented with on your behalf.',
        target: '[data-tutorial="tab-metrics"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
    ],
  },
  {
    id: 'from-scratch',
    title: 'Build From Scratch',
    blurb: 'Drag your own boxes onto the canvas, wire them together by hand, and train your own design.',
    difficulty: 'advanced',
    minutes: 10,
    steps: [
      {
        title: 'Ready for full control?',
        body:
          "Everything so far has been built for you. Now let's place boxes by hand and connect them yourself " +
          '— exactly what BrainBuilder does automatically, just with you in the driver\'s seat. Start with a ' +
          'clean canvas (use the toolbar to clear it, or open a fresh graph).',
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'Drag a box onto the canvas',
        body:
          'Drag any block from the Components shelf onto the empty canvas, or double-click it to drop it in ' +
          'the middle. Try starting with a "linear" box — one of the most basic building blocks there is.',
        target: '[data-tutorial="palette"]',
      },
      {
        title: 'Add a second box',
        body:
          'Drag out an "activation" box (like "relu" or "gelu") next. Activations are what let a neural ' +
          'network learn curved, complicated patterns instead of only straight lines — almost every real ' +
          'model alternates layer, activation, layer, activation.',
        target: '[data-tutorial="palette"]',
      },
      {
        title: 'Connect the dots',
        body:
          "Drag from one box's output dot to another box's input dot to connect them. BrainBuilder checks " +
          'that the shapes match, so you\'ll know right away if two boxes don\'t fit together.',
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'Keep going',
        body:
          'Repeat: add a box, connect it, check the shape. A real model is usually 4-20 boxes chained ' +
          'together. There\'s no single "correct" answer — different arrangements can all work, some better ' +
          'than others for a given job.',
        target: '[data-tutorial="palette"]',
      },
      {
        title: 'Validate before training',
        body:
          'Click "Validate" on the canvas toolbar any time to check your wiring is complete and correct, ' +
          'before spending time training it.',
        target: '[data-tutorial="validate-btn"]',
      },
      {
        title: 'Export & Train your own design',
        body:
          'Once validation passes, hit "Export & Train" and watch the Metrics tab. You designed this network ' +
          "yourself, box by box — however it performs, that's genuinely your architecture at work.",
        target: '[data-tutorial="export-train-btn"]',
      },
    ],
  },
  {
    id: 'from-scratch-text',
    title: 'Build a Text Model From Scratch',
    blurb: 'Hand-wire the same "words become numbers" pipeline you saw in Look Inside a Text Model.',
    difficulty: 'advanced',
    minutes: 8,
    steps: [
      {
        title: 'Text needs one extra first step',
        body:
          'Everything from "Build From Scratch" still applies — drag, connect, validate, train. Text just ' +
          "needs one thing pictures and plain numbers don't: a translation step from words into numbers, " +
          'before any of the usual boxes can do their job.',
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'Start with embedding',
        body:
          'Drag out an "embedding" box first. This is that translation step — it looks up each word in a ' +
          'table and hands back a list of numbers for it. Open its settings in the Inspector: "vocab size" ' +
          "is how many different words it's allowed to know.",
        target: '[data-tutorial="palette"]',
      },
      {
        title: 'Add attention to mix word meanings together',
        body:
          'Drag out an "attention" box and connect the embedding\'s output into it. Attention lets each ' +
          'word\'s numbers get adjusted based on the other words nearby — it\'s how the model tells "bank" ' +
          '(the river) apart from "bank" (the money) using context.',
        target: '[data-tutorial="palette"]',
      },
      {
        title: 'Finish with a linear box',
        body:
          'Drag out a "linear" box and connect attention\'s output into it — this is the same kind of box ' +
          'you used for pictures. Its "out features" setting should match how many categories you\'re ' +
          'sorting into (2 for yes/no, more for multiple categories).',
        target: '[data-tutorial="palette"]',
      },
      {
        title: 'Validate, then train',
        body:
          'Click "Validate" to check every connection lines up, then "Export & Train" and watch the Metrics ' +
          'tab. Same finish line as every other model — you just chose a different starting pipeline for ' +
          'text\'s particular shape of data.',
        target: '[data-tutorial="validate-btn"]',
      },
    ],
  },
  {
    id: 'from-scratch-regression',
    title: 'Build a Regression Model From Scratch',
    blurb: 'Hand-wire a number-predicting model, paying attention to the one box that has to be different.',
    difficulty: 'advanced',
    minutes: 6,
    steps: [
      {
        title: 'Almost identical to classification — with one catch',
        body:
          "Build the body the same way you would for any model: drag out a \"linear\" box, then an activation " +
          '("relu" or "gelu"), and repeat that pair a couple of times, connecting each one\'s output to the ' +
          "next one's input.",
        target: '[data-tutorial="palette"]',
      },
      {
        title: 'The last box is special',
        body:
          "For the final box, drag out one more \"linear\" box and set its \"out features\" to exactly 1 — " +
          'one number out, since you\'re predicting a single value, not choosing between categories.',
        target: '[data-tutorial="tab-inspector"]',
        focusTab: { slot: 'side', tabId: 'inspector' },
      },
      {
        title: "Don't add an activation after it",
        body:
          'Unlike every other box, the very last one should feed straight out with no activation box after ' +
          'it. Activations squash numbers into a fixed range (like 0 to 1) — great for a "how confident is ' +
          "this a cat\" score, but wrong for a price or temperature that needs to be any real number.",
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'Validate, then train',
        body:
          'Click "Validate" to make sure everything connects cleanly, then "Export & Train". Watch the loss ' +
          'on the Metrics tab shrink the same way it always does — the only thing that changed is what that ' +
          "last box hands back.",
        target: '[data-tutorial="validate-btn"]',
      },
    ],
  },
  {
    id: 'synthesize-component',
    title: 'Invent a New Building Block',
    blurb: "Describe a layer that doesn't exist yet, and watch BrainBuilder write, test, and add it for you.",
    difficulty: 'advanced',
    minutes: 6,
    steps: [
      {
        title: "What if the box you need doesn't exist?",
        body:
          "Every box you've used so far — linear, embedding, attention — was hand-built ahead of time. The " +
          'Synthesize tab does something different: it writes a brand-new box from a plain-English ' +
          'description, using an AI language model, then proves it actually works before trusting it.',
        target: '[data-tutorial="tab-synthesize"]',
        focusTab: { slot: 'side', tabId: 'synthesize' },
      },
      {
        title: 'Describe the box you want',
        body:
          'Type a description of a layer that doesn\'t exist in your palette — for example "a swish ' +
          'activation: x times sigmoid of x, same shape in and out". Be specific about the math and the ' +
          "shape, the same way you'd describe it to a person.",
        target: '[data-tutorial="synth-description"]',
      },
      {
        title: 'Click Synthesize and wait',
        body:
          "BrainBuilder asks the language model for both a description (what the box's settings and " +
          "plugs look like) and real code, then runs everything through a sandboxed smoke test — a tiny fake " +
          "input, checked for the right shape coming out — before it's ever trusted.",
        target: '[data-tutorial="synth-button"]',
      },
      {
        title: 'Read the result',
        body:
          "\"Smoke test passed\" means the generated code ran safely and produced the right shape — safe to " +
          'add. "Failed" means BrainBuilder caught a problem itself and refuses to add it, protecting you ' +
          "from broken AI-written code without you having to read a line of it yourself.",
        target: '[data-tutorial="synth-result"]',
      },
      {
        title: 'Add it to your palette',
        body:
          'If the smoke test passed, click "Add to canvas" — your new box appears in the Components shelf ' +
          "immediately, right alongside the built-in ones, ready to wire into any model like any other box.",
        target: '[data-tutorial="synth-accept-btn"]',
      },
    ],
  },
  {
    id: 'self-building-agent',
    title: 'Let the AI Improve BrainBuilder Itself',
    blurb: 'Hand a coding task to an AI agent that edits BrainBuilder\'s own code, safely, in a sandbox.',
    difficulty: 'advanced',
    minutes: 7,
    steps: [
      {
        title: 'The most advanced tool in the app',
        body:
          "Everything else in BrainBuilder builds models. The Agent tab is different — it points an AI coding " +
          "agent at BrainBuilder's own source code and lets it make real changes, inside a sandboxed worktree " +
          "so nothing it does can damage your actual project without your say-so.",
        target: '[data-tutorial="tab-agent"]',
        focusTab: { slot: 'side', tabId: 'agent' },
      },
      {
        title: 'Choose how much trust to give it',
        body:
          '"Propose + approve" shows you every change before anything merges — the safest choice, and the ' +
          'default. "Auto-apply" merges automatically whenever its own tests pass, but you can always revert. ' +
          '"Full autonomy" gives it the most freedom and the least oversight — powerful, but opt-in for a ' +
          'reason.',
        target: '[data-tutorial="agent-mode"]',
      },
      {
        title: 'Start a session',
        body:
          'Click "Start session". BrainBuilder creates a fresh worktree — a separate, safe copy of the project ' +
          "— so the agent's work never touches your real files until you explicitly approve it.",
        target: '[data-tutorial="agent-start-btn"]',
      },
      {
        title: 'Describe a real task',
        body:
          'Type something concrete and small, like "add a swish activation component with a smoke test" — ' +
          'the same kind of thing you just did by hand in "Invent a New Building Block", but this time the ' +
          'agent writes the actual code changes itself.',
        target: '[data-tutorial="agent-task"]',
      },
      {
        title: 'Run it and watch',
        body:
          'Click "Run task". You\'ll see the agent\'s progress, then the diff of what it changed, then whether ' +
          "its own tests passed — nothing merges into your real checkout until the tests are green and " +
          '(in Propose + approve mode) you say yes.',
        target: '[data-tutorial="agent-run-btn"]',
      },
      {
        title: 'Approve, or throw it away',
        body:
          '"Approve + merge" brings the agent\'s changes into your real project. "Discard" deletes the whole ' +
          'worktree instead — as if the agent had never run — no trace, no risk, any time you\'re not happy ' +
          'with what it did.',
        target: '[data-tutorial="agent-approve-btn"]',
      },
    ],
  },
  {
    id: 'distributed-training',
    title: 'Train Across Multiple Devices',
    blurb: 'Pair your devices into a Cluster and split one training run across all of them at once.',
    difficulty: 'advanced',
    minutes: 7,
    steps: [
      {
        title: 'One model, several computers',
        body:
          "Every model so far trained on this one device. The Cluster tab lets several devices — other " +
          "computers on the same network — train the exact same model together, each handling a slice of the " +
          "data and combining what they learn every round. This needs at least one other device to try for " +
          "real, but the setup works the same either way.",
        target: '[data-tutorial="tab-cluster"]',
        focusTab: { slot: 'bottom', tabId: 'cluster' },
      },
      {
        title: 'Create your Cluster',
        body:
          'Give this device a name and click "Create My Cluster" — this device becomes the Manager, the one ' +
          "that coordinates everyone else. Every device you add later joins this same Cluster.",
        target: '[data-tutorial="cluster-create-btn"]',
      },
      {
        title: 'Invite another device',
        body:
          'As the Manager, click "Generate Pairing Code" — a 6-digit code valid for 5 minutes. Type that code ' +
          'into the "Join" box on the other device\'s Cluster tab (using this same BrainBuilder app) to add ' +
          'it.',
        target: '[data-tutorial="cluster-pairing-btn"]',
      },
      {
        title: 'See who\'s connected',
        body:
          "Every paired device shows up in this list, along with its hardware — CPU cores and RAM. This is " +
          "how you check everyone actually joined before starting a training run.",
        target: '[data-tutorial="cluster-devices"]',
      },
      {
        title: '(Optional) Check it from your phone',
        body:
          "If an address shows up under \"Phone / Tablet Access\", open it in any browser on the same Wi-Fi " +
          "to watch the Cluster's live status — no app install needed, view-only, handy for checking progress " +
          "without sitting at the computer.",
        target: '[data-tutorial="cluster-observer"]',
      },
      {
        title: 'Host a training run on the Cluster',
        body:
          'Build a model on the canvas like normal, set how many other devices to wait for, then click "Host ' +
          'Training on Cluster". Each joined device works on its own slice of the data and everyone\'s real ' +
          'gradients get averaged together every round — genuinely faster training, not a simulation.',
        target: '[data-tutorial="cluster-host-btn"]',
      },
      {
        title: 'Watch it train',
        body:
          "Switch to the Metrics tab — the loss chart updates the exact same way a single-device run does. " +
          "Training across a Cluster looks identical from here; the only difference is how many machines are " +
          "doing the work underneath.",
        target: '[data-tutorial="tab-metrics"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
    ],
  },
];
