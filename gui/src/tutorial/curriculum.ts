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
  {
    id: 'model-hub',
    title: 'Use Models You Already Have',
    blurb: "Load models straight off your computer — no downloads, no accounts, no network at all.",
    difficulty: 'intermediate',
    minutes: 6,
    steps: [
      {
        title: 'Your computer probably already has models on it',
        body:
          "If you've ever used another AI tool, there's a good chance model files are already sitting on your " +
          "disk somewhere. The Models tab finds them — safetensors, GGUF, ONNX, or PyTorch — without " +
          "downloading anything or talking to the internet.",
        target: '[data-tutorial="tab-models"]',
        focusTab: { slot: 'side', tabId: 'models' },
      },
      {
        title: 'Point it at a folder',
        body:
          'Type the path to a folder that has model files in it and click "Add folder" — for example wherever ' +
          "another tool caches its downloads. BrainBuilder scans it and remembers it for next time.",
        target: '[data-tutorial="modelhub-add-dir"]',
      },
      {
        title: 'See what it found',
        body:
          'Each model shows its name and which formats it has — safetensors, GGUF, ONNX, PyTorch. These little ' +
          'labels are chips, not downloads: everything here was already on your computer before you opened ' +
          'this tab.',
        target: '[data-tutorial="modelhub-list"]',
      },
      {
        title: 'Look inside a model',
        body:
          'Click "Inspect" on any model to see its actual tensor names, shapes, and data types — the same ' +
          'kind of detail you\'d need to wire it in as a transfer-learning backbone (like in the very first ' +
          'tutorial\'s optional pretrained-model step).',
        target: '[data-tutorial="modelhub-inspect-btn"]',
      },
      {
        title: '(If GGUF) Register it with Ollama',
        body:
          'A GGUF-format model can be "Register"ed — this hands it to Ollama so you can chat with it from the ' +
          'command line, completely separate from anything you build on the canvas.',
        target: '[data-tutorial="modelhub-register-btn"]',
      },
      {
        title: 'Use one as a transfer-learning backbone',
        body:
          'Back in the Build tab, turn on "Start from a pretrained model" and point it at a local ' +
          '.safetensors file — the tensor name you need is exactly what "Inspect" just showed you a moment ' +
          'ago.',
        target: '[data-tutorial="intent-transfer"]',
        focusTab: { slot: 'side', tabId: 'build' },
      },
      {
        title: '(Bonus) Pick which GPU trains',
        body:
          "Further down this same tab, the GPU picker lists every graphics card BrainBuilder can use and lets " +
          'you pin a preferred one — handy on a machine with more than one, like a laptop with both an ' +
          'integrated and a dedicated GPU. "Test this GPU" confirms it actually binds before you rely on it.',
        target: '[data-tutorial="gpu-select"]',
        focusTab: { slot: 'side', tabId: 'models' },
      },
    ],
  },
  {
    id: 'checkpoints-and-saving',
    title: 'Save Your Progress, Come Back Later',
    blurb: "What actually gets saved when you train, and how to pick up right where you left off.",
    difficulty: 'intermediate',
    minutes: 5,
    steps: [
      {
        title: 'Training saves itself automatically',
        body:
          "You've never had to click a \"save\" button after training — BrainBuilder writes what your model " +
          'learned to a checkpoint file on its own, every time a run finishes. Think of it as an automatic ' +
          'save-game for your model\'s learned numbers.',
        target: '[data-tutorial="tab-predict"]',
        focusTab: { slot: 'bottom', tabId: 'predict' },
      },
      {
        title: 'Predict notices on its own',
        body:
          'Before you\'ve trained anything, Predict shows "No trained checkpoint yet for this graph". Once ' +
          "training finishes, this message disappears within a few seconds by itself — BrainBuilder quietly " +
          "checks in the background, so you never need to switch tabs and back to refresh it.",
        target: '[data-tutorial="predict-empty"]',
      },
      {
        title: 'Try a real prediction',
        body:
          'Once trained, click "Run on first 5 rows" to see your checkpoint actually being used — it loads ' +
          "those saved numbers back in and runs them on real data, without retraining anything.",
        target: '[data-tutorial="predict-run-btn"]',
      },
      {
        title: 'Save your graph for later',
        body:
          'Click "Save…" on the canvas toolbar to write your graph\'s structure to a file. This saves what ' +
          "the model looks like — its boxes and connections — as a separate thing from the checkpoint, which " +
          'holds what it learned.',
        target: '[data-tutorial="graph-save-btn"]',
      },
      {
        title: 'Reopen it and pick up where you left off',
        body:
          'Click "Load…" and choose that file — even after closing and reopening BrainBuilder entirely. As ' +
          "long as the checkpoint file is still on your computer, Predict works again immediately, with zero " +
          "retraining, because loading restores the exact same model identity the checkpoint was saved under.",
        target: '[data-tutorial="graph-load-btn"]',
      },
      {
        title: 'This is why "New" asks you to confirm',
        body:
          'Clicking "New" (which you may remember asks you to confirm first) starts a completely fresh model ' +
          'identity. Your old checkpoint file isn\'t deleted, but nothing in the app can find it anymore under ' +
          'the new blank canvas — which is exactly the situation that confirmation is there to prevent.',
        target: '[data-tutorial="graph-new-btn"]',
      },
    ],
  },
  {
    id: 'hyperparameter-deep-dive',
    title: 'Hyperparameters, Up Close',
    blurb: "Every box has its own dials, and the Data tab has whole-run dials too — see both and what they control.",
    difficulty: 'intermediate',
    minutes: 6,
    steps: [
      {
        title: 'Every box has its own settings',
        body:
          'Every box you drop on the canvas has its own hyperparameters — numbers and switches that shape how ' +
          'that one box behaves. The Inspector tab is where you see and change them for whichever box is ' +
          'currently selected.',
        target: '[data-tutorial="tab-inspector"]',
        focusTab: { slot: 'side', tabId: 'inspector' },
      },
      {
        title: 'Nothing selected yet?',
        body:
          'Inspector stays empty until you pick a box. Click any box on your canvas to select it — its name ' +
          "lights up and its settings appear here. If you've already got one selected, you'll see its dials " +
          'instead of this message, which is fine too.',
        target: '[data-tutorial="inspector-empty"]',
      },
      {
        title: 'Dials appear automatically',
        body:
          "Once a box is selected, its dials show up here — pulled straight from that component's own " +
          'definition. A convolution box shows convolution settings; an attention box shows attention ' +
          "settings. You'll never see a setting that doesn't actually apply to the box you picked.",
        target: '[data-tutorial="inspector-form"]',
      },
      {
        title: "Three kinds of dials, none of them breakable",
        body:
          'A number field takes a count or a rate. A checkbox is a plain on/off switch. A dropdown offers a ' +
          "fixed list of choices. Because switches and dropdowns can't hold an invalid value, and number " +
          "fields only accept numbers, there's no way to type in a setting that would break your model.",
        target: '[data-tutorial="inspector-form"]',
      },
      {
        title: 'A different set of dials for the whole run',
        body:
          "These per-box dials are separate from the ones on the Data tab — learning rate, epochs, batch " +
          "size, optimizer. Those apply to the entire training run, not to one box. Both are called " +
          "hyperparameters; one is per-component, the other is per-run.",
        target: '[data-tutorial="training-hyperparams"]',
        focusTab: { slot: 'side', tabId: 'data' },
      },
      {
        title: "Don't want to guess either kind by hand?",
        body:
          "You don't have to. Auto-tune, right here on the Data tab, experiments with the whole-run settings " +
          "for you and applies whichever combination trained best — there's a dedicated tutorial that walks " +
          'through it step by step if you want the full picture.',
        target: '[data-tutorial="autotune-btn"]',
      },
    ],
  },
  {
    id: 'data-preprocessing',
    title: 'Clean Up Your Data Automatically',
    blurb: 'Add reusable steps — like rescaling a column — that run automatically every time you train or predict.',
    difficulty: 'intermediate',
    minutes: 5,
    steps: [
      {
        title: 'Raw data is rarely ready as-is',
        body:
          "One column might range from 0 to 1, another from 0 to 1,000,000 — that mismatch can make training " +
          'slower or less accurate. The Data tab has a spot for automatic cleanup steps that fix this kind of ' +
          "thing, every single time you train or predict, without editing your original file.",
        target: '[data-tutorial="tab-data"]',
        focusTab: { slot: 'side', tabId: 'data' },
      },
      {
        title: 'Nothing happens by default',
        body:
          "Until you add a step, your data trains exactly as it appears in the file — untouched. Preprocessing " +
          "is entirely optional and only runs if you ask for it. If you've already got a step added, you'll " +
          'see the list instead of this message, which is fine too.',
        target: '[data-tutorial="preprocess-empty"]',
      },
      {
        title: 'Two things you can do to a column',
        body:
          '"normalize" rescales a column so its numbers average to zero and spread out evenly — good for ' +
          'columns with very different ranges. "cast" changes a column\'s type, like turning whole numbers ' +
          "into decimal numbers — useful when a column needs to be a specific type to train correctly.",
        target: '[data-tutorial="preprocess-op-select"]',
      },
      {
        title: 'Type the exact column name',
        body:
          "Type the name of the column this step applies to, exactly as it appears in the preview table above " +
          '— start typing and matching column names will suggest themselves if you\'ve already picked a ' +
          'dataset.',
        target: '[data-tutorial="preprocess-column-input"]',
      },
      {
        title: 'Add it',
        body:
          '"Add step" appends it to the list immediately — no separate save button, and nothing about your ' +
          "original data file changes. You can add as many steps as you need.",
        target: '[data-tutorial="preprocess-add-btn"]',
      },
      {
        title: 'Steps run in order, every time',
        body:
          'Every step in this list runs top to bottom, automatically, both when you train and when you ' +
          'predict — so the same cleanup always happens the same way, and you never have to remember to redo ' +
          'it by hand.',
        target: '[data-tutorial="preprocess-list"]',
      },
      {
        title: 'Changed your mind? Remove it',
        body:
          'Click the ✕ next to any step to remove it — takes effect immediately, and like adding a step, it ' +
          "never touches your original file. There's no wrong way to experiment here.",
        target: '[data-tutorial="preprocess-remove-btn"]',
      },
    ],
  },
  {
    id: 'image-transforms',
    title: 'Getting More Out of Your Photos',
    blurb: 'Resize, grayscale, and flip settings for a folder of images — what they do and when to use them.',
    difficulty: 'intermediate',
    minutes: 5,
    steps: [
      {
        title: 'Turning photos into numbers a model can learn from',
        body:
          "When your data is a folder of photos, BrainBuilder needs a few consistent settings to turn every " +
          'image into the same shape of numbers before training. These appear once you pick "A folder of ' +
          'images" as your data type.',
        target: '[data-tutorial="intent-data"]',
        focusTab: { slot: 'side', tabId: 'build' },
      },
      {
        title: 'Every photo, the same size',
        body:
          '"Resize to __ px" scales every photo to the same square size before training — models need every ' +
          'example to be exactly the same shape. Smaller (like 32px) trains fast on any computer; bigger keeps ' +
          'more detail but takes longer.',
        target: '[data-tutorial="intent-image-options"]',
      },
      {
        title: 'Color costs more to learn from',
        body:
          'A color photo carries three numbers per pixel — red, green, and blue. Checking "Grayscale" ' +
          "collapses that to one number per pixel, which trains faster and uses less memory. Only turn it on " +
          "if color genuinely doesn't matter for telling your classes apart (handwriting, yes — ripe vs. " +
          'unripe fruit, no).',
        target: '[data-tutorial="intent-grayscale"]',
      },
      {
        title: 'More variety without more photos',
        body:
          '"Flip every other photo" mirrors half your images left-to-right before training. It\'s a real ' +
          "trick called data augmentation — it teaches the model what your subject looks like in general, not " +
          "just facing one particular direction, without you needing to take a single extra picture.",
        target: '[data-tutorial="intent-augment"]',
      },
      {
        title: 'These apply the moment you build',
        body:
          'Resize, grayscale, and flip are all baked in the instant you click "Build my model" — there\'s ' +
          "nothing further to configure. Change your mind later by adjusting these and building again.",
        target: '[data-tutorial="intent-build-btn"]',
      },
      {
        title: 'Nothing here touches your real photos',
        body:
          'Every one of these settings only changes how BrainBuilder reads your images into memory for ' +
          "training — your original photo files on disk are never modified, resized, or overwritten. Feel " +
          'free to experiment.',
        target: '[data-tutorial="intent-image-options"]',
      },
    ],
  },
  {
    id: 'reading-metrics',
    title: 'Is My Training Actually Working?',
    blurb: 'Read the live loss chart and let BrainBuilder tell you, in plain English, what to do next.',
    difficulty: 'intermediate',
    minutes: 5,
    steps: [
      {
        title: 'Where to watch training happen',
        body:
          'Every time you click "Export & Train," this tab lights up with a live chart and plain-English ' +
          "notes about how it's going — no need to guess whether things are working.",
        target: '[data-tutorial="tab-metrics"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
      {
        title: 'The loss chart',
        body:
          "This line is the loss — a single score for how wrong the model's guesses currently are. It should " +
          "trend downward as training goes; a chart that's flat or climbing means something's off, and " +
          'BrainBuilder will tell you plainly (more on that in a moment).',
        target: '[data-tutorial="metrics-chart"]',
      },
      {
        title: 'The numbers underneath',
        body:
          'Epoch is which full pass through your data you\'re on; Step counts individual training steps ' +
          'within it; Loss is the current score. Watching these tick up confirms training is actually ' +
          "running, not stuck.",
        target: '[data-tutorial="metrics-latest"]',
      },
      {
        title: 'The quick verdict',
        body:
          'This line does the math for you: a percentage lower than where loss started means training is ' +
          'working, while a plain warning that loss is increasing means something needs attention — most ' +
          'often the learning rate.',
        target: '[data-tutorial="metrics-trend"]',
      },
      {
        title: 'When something needs fixing',
        body:
          'A few seconds after training pauses or finishes, BrainBuilder reads the whole curve and tells you, ' +
          'in plain language, what happened and what to try: "training diverged" (the numbers blew up — ' +
          'lower the learning rate), "loss barely changed" (try a higher learning rate, more epochs, or a ' +
          'bigger model), or reassurance that training is going well.',
        target: '[data-tutorial="metrics-diagnostics"]',
      },
      {
        title: "Keep seeing the same warning?",
        body:
          "If the diagnostics keep pointing at the learning rate, you don't have to guess a new number by " +
          'hand — Auto-tune (on the Data tab) runs several short trials and applies whichever settings ' +
          'actually trained best.',
        target: '[data-tutorial="autotune-btn"]',
        focusTab: { slot: 'side', tabId: 'data' },
      },
    ],
  },
  {
    id: 'comparing-experiments',
    title: 'Which Settings Actually Worked Best?',
    blurb: 'Every training run is logged automatically — compare past attempts instead of remembering them.',
    difficulty: 'intermediate',
    minutes: 5,
    steps: [
      {
        title: 'Every run, remembered automatically',
        body:
          'Every time you click "Export & Train," BrainBuilder quietly writes down what happened — no setup, ' +
          'nothing to turn on. This tab is where you go to see it.',
        target: '[data-tutorial="tab-experiments"]',
        focusTab: { slot: 'bottom', tabId: 'experiments' },
      },
      {
        title: 'Nothing yet?',
        body:
          "Training has to happen at least once before anything shows up here. If you've already trained " +
          "something, you'll see a table instead of this message — that's expected too.",
        target: '[data-tutorial="experiments-empty"]',
      },
      {
        title: 'Catching up on a run in progress',
        body:
          "If you started training from another tab and just switched over, click \"Refresh\" to pull in " +
          "that run's result without reloading the whole app.",
        target: '[data-tutorial="experiments-refresh-btn"]',
      },
      {
        title: 'Reading a row',
        body:
          'Each row is one real training run, newest first: the exact loss function, optimizer, learning ' +
          'rate, and epoch count you used, plus how loss moved from first to last — so two runs of the same ' +
          'model with different settings sit right next to each other for comparison.',
        target: '[data-tutorial="experiments-table"]',
      },
      {
        title: "Why this is separate from checkpoints",
        body:
          'Training the same model again overwrites its one saved checkpoint — but every run still gets its ' +
          "own row here, forever. That's the difference: a checkpoint is what your model currently is, while " +
          "this history is everything you've ever tried.",
        target: '[data-tutorial="experiments-table"]',
      },
      {
        title: "Don't want to fill this table in by hand?",
        body:
          "Auto-tune (on the Data tab) runs several short trials in one go and applies whichever combination " +
          'trained best — handy before you\'ve settled on a design worth logging here for the long run.',
        target: '[data-tutorial="autotune-btn"]',
        focusTab: { slot: 'side', tabId: 'data' },
      },
    ],
  },
  {
    id: 'export-and-deploy',
    title: 'Taking Your Model Outside BrainBuilder',
    blurb: 'Export your trained checkpoint as a standard file you can use in your own scripts or projects.',
    difficulty: 'intermediate',
    minutes: 4,
    steps: [
      {
        title: "What \"deployment\" actually means",
        body:
          "Deployment just means getting your trained model out of BrainBuilder and into wherever you " +
          "actually want to use it — a script, a website, another program. This tab is where that hand-off " +
          'happens.',
        target: '[data-tutorial="tab-predict"]',
        focusTab: { slot: 'bottom', tabId: 'predict' },
      },
      {
        title: 'Train first',
        body:
          "You'll need a trained checkpoint before there's anything to export. If you've already trained " +
          "this model, you'll see an export button instead of this message — that's expected too.",
        target: '[data-tutorial="predict-empty"]',
      },
      {
        title: 'Export it',
        body:
          '"Export checkpoint…" opens a save dialog — pick anywhere on your computer. That copy is now ' +
          "completely independent of BrainBuilder's internal files.",
        target: '[data-tutorial="export-checkpoint-btn"]',
      },
      {
        title: 'No special format, no lock-in',
        body:
          "What you get is a standard PyTorch checkpoint — a plain file any Python script with PyTorch " +
          'installed can open with `torch.load(...)`. Nothing about it is proprietary to BrainBuilder, and ' +
          'nothing further is required to read it elsewhere.',
        target: '[data-tutorial="export-checkpoint-btn"]',
      },
      {
        title: "Exporting doesn't remove anything",
        body:
          "It's a copy, not a move — your model stays exactly as usable inside BrainBuilder afterward, for " +
          'more predictions or further training, as it was before you exported.',
        target: '[data-tutorial="predict-run-btn"]',
      },
      {
        title: 'Remember: two files, two purposes',
        body:
          "The checkpoint (what you just exported) is what your model learned. The graph file from \"Save…\" " +
          "is its structure — its boxes and connections. Deploying elsewhere only needs the checkpoint; " +
          'resuming work in BrainBuilder later needs both.',
        target: '[data-tutorial="graph-save-btn"]',
      },
    ],
  },
  {
    id: 'troubleshooting',
    title: 'When Something Doesn\'t Work',
    blurb: 'Where to look when a connection fails, training errors out, or a component misbehaves.',
    difficulty: 'intermediate',
    minutes: 6,
    steps: [
      {
        title: 'Catch mistakes before you train',
        body:
          'Click "Validate" any time to check your wiring — every required input connected, every shape ' +
          'matching up — in a fraction of a second, without running any real training. There\'s no reason not ' +
          'to check before committing to a full run.',
        target: '[data-tutorial="validate-btn"]',
      },
      {
        title: "Some mistakes can't even be made",
        body:
          "Try to connect two boxes that don't fit — wrong shape, already-wired input, a box connecting to " +
          "itself — and BrainBuilder refuses the connection outright and explains why. You can't accidentally " +
          "build something that was never going to work in the first place.",
        target: '[data-tutorial="canvas"]',
      },
      {
        title: 'One place for everything that goes wrong',
        body:
          'A rejected connection, a failed validation, a training error, a failed prediction — all of it ' +
          'lands in the same place. When something doesn\'t work, this tab is always your first stop.',
        target: '[data-tutorial="tab-console"]',
        focusTab: { slot: 'bottom', tabId: 'console' },
      },
      {
        title: 'Reading an error',
        body:
          "Every message is timestamped and stays until you clear it, so you can scroll back and see exactly " +
          "what happened and in what order — useful when several things went wrong in a row and you need to " +
          'find the very first one.',
        target: '[data-tutorial="console-output"]',
      },
      {
        title: 'Clearing the log is safe',
        body:
          '"Clear" only tidies up what you\'re looking at — it never undoes anything, changes your model, or ' +
          'affects training. Clear it whenever the list gets long.',
        target: '[data-tutorial="console-clear-btn"]',
      },
      {
        title: 'A deeper layer: sandboxed component calls',
        body:
          "Every component's code — even ones an AI wrote for you — runs in a locked-down sandbox " +
          "that can only touch what it's explicitly allowed to. The \"Nervous System\" tab shows every one of " +
          "those calls, useful when you suspect a specific component's code rather than how it's wired.",
        target: '[data-tutorial="tab-audit"]',
      },
      {
        title: 'Reading an audit entry',
        body:
          'Each entry shows whether a sandboxed call was allowed, denied for touching something it wasn\'t ' +
          "permitted to, or killed for running too long — plain evidence of what a component actually did, " +
          'never a silent mystery.',
        target: '[data-tutorial="console-audit"]',
      },
    ],
  },
  {
    id: 'feature-importance',
    title: 'Why Did It Say That?',
    blurb: 'Ask your trained model which columns of your data it actually pays attention to.',
    difficulty: 'intermediate',
    minutes: 5,
    steps: [
      {
        title: 'A deeper question than "what\'s the answer"',
        body:
          "Once a model is trained, you can ask it something more interesting than just a prediction — " +
          '"why did you say that?" This tab is where you ask.',
        target: '[data-tutorial="tab-predict"]',
        focusTab: { slot: 'bottom', tabId: 'predict' },
      },
      {
        title: 'A real test, not a guess',
        body:
          '"Explain this model…" runs a genuine experiment against your trained model: it scrambles one ' +
          "column of your real data at a time and checks how much that changes the model's answers. Nothing " +
          "here is simulated or estimated — it's your actual model, actually tested.",
        target: '[data-tutorial="explain-btn"]',
      },
      {
        title: 'Reading the ranking',
        body:
          'The result is a ranked list: columns near the top are ones the model leans on heavily — scrambling ' +
          "them changes its answers a lot. Columns near the bottom barely move the needle, meaning the model " +
          "isn't really using them, whether or not you expected it to.",
        target: '[data-tutorial="explain-results"]',
      },
      {
        title: 'When the ranking surprises you',
        body:
          "If a column you expected to matter shows up near the bottom, or one you ignored shows up at the " +
          "top, that's worth investigating — it can reveal a data problem (like a leaked answer hiding in an " +
          'unexpected column) as easily as it reveals something genuinely interesting about your data.',
        target: '[data-tutorial="explain-results"]',
      },
      {
        title: 'Tabular data only, for now',
        body:
          'This works on spreadsheet-style data, where every column has a real name to rank. Image and text ' +
          "models don't have that kind of column to point at, so this tool only appears for spreadsheet " +
          '(CSV/Parquet) sources.',
        target: '[data-tutorial="predict-run-btn"]',
      },
    ],
  },
  {
    id: 'checkpoint-versioning',
    title: 'Undoing a Bad Training Run',
    blurb: 'Training over a working model used to be permanent. Now every version it replaces is kept.',
    difficulty: 'intermediate',
    minutes: 5,
    steps: [
      {
        title: 'Training again used to be a one-way door',
        body:
          "Retraining a model you'd already trained used to silently overwrite the only saved checkpoint — " +
          'if the new run made things worse, the old, working version was just gone. Not anymore: this tab ' +
          'keeps every version a training run replaces, automatically.',
        target: '[data-tutorial="tab-predict"]',
        focusTab: { slot: 'bottom', tabId: 'predict' },
      },
      {
        title: 'Nothing here yet?',
        body:
          "Version history only has something to show once you've trained the same model more than once — " +
          "a model's very first checkpoint has nothing earlier to compare against. Train it again and this " +
          "list stops being empty, with no setup required.",
        target: '[data-tutorial="versions-empty"]',
      },
      {
        title: 'Reading the list',
        body:
          'Each entry is a real checkpoint that used to be the current one, newest first, with exactly when ' +
          "it was replaced and how big the file is — so you can tell your versions apart even without " +
          'renaming anything.',
        target: '[data-tutorial="versions-list"]',
      },
      {
        title: 'Bringing one back',
        body:
          '"Restore" asks you to confirm, then makes that version current again. And because restoring ' +
          "archives whatever it replaces too, undoing a bad training run is never a one-way door — and " +
          "neither is undoing the undo.",
        target: '[data-tutorial="restore-version-btn"]',
      },
      {
        title: 'Pair this with the History tab',
        body:
          "Version history tells you what you can go back to; the History tab (from \"Which Settings " +
          "Actually Worked Best?\") tells you which run is worth going back to — the settings and loss for " +
          'every attempt, side by side.',
        target: '[data-tutorial="tab-experiments"]',
        focusTab: { slot: 'bottom', tabId: 'experiments' },
      },
      {
        title: 'For safety beyond this computer',
        body:
          "Version history lives inside BrainBuilder's own folder, alongside your current checkpoint. For a " +
          'copy that survives even if BrainBuilder itself is ever removed, "Export checkpoint…" on the ' +
          'Predict tab saves one to anywhere you choose.',
        target: '[data-tutorial="export-checkpoint-btn"]',
        focusTab: { slot: 'bottom', tabId: 'predict' },
      },
    ],
  },
  {
    id: 'comparing-architectures',
    title: 'Is a Bigger Model Actually Better?',
    blurb: 'Build two genuinely different designs on the same data and let the History tab settle it.',
    difficulty: 'intermediate',
    minutes: 6,
    steps: [
      {
        title: "Not just settings — shapes, too",
        body:
          "The History tab doesn't just compare learning rates and optimizers anymore — it shows the actual " +
          'shape of each model, so you can compare completely different designs side by side, not just ' +
          'different settings on the same one.',
        target: '[data-tutorial="tab-experiments"]',
        focusTab: { slot: 'bottom', tabId: 'experiments' },
      },
      {
        title: 'Reading an architecture',
        body:
          '"linear → relu → linear" reads left to right, exactly like the boxes on your canvas: this model ' +
          "has three boxes chained together. A row that just says \"linear\" is a single-box model — simpler, " +
          "by definition, whatever its score turns out to be.",
        target: '[data-tutorial="experiments-architecture"]',
      },
      {
        title: 'The actual experiment',
        body:
          'Build two versions of the same idea on the canvas — say, a single "linear" box, then a second ' +
          'attempt with "linear → relu → linear" added — pointing at the exact same dataset. Train both. ' +
          "Each becomes its own row here, ready to compare.",
        target: '[data-tutorial="palette"]',
      },
      {
        title: "Bigger isn't automatically better",
        body:
          "Sometimes the deeper architecture wins by a wide margin. Sometimes the simple one matches it with " +
          "far less training time, or even beats it on too little data. Either result is a genuinely useful " +
          "thing to learn about your specific problem — not a wrong answer.",
        target: '[data-tutorial="experiments-table"]',
      },
      {
        title: 'Keep it a fair comparison',
        body:
          "For the comparison to mean anything, change one thing at a time. If you change both the " +
          "architecture and the dataset between runs, you won't know which one caused the difference in " +
          "score — the architecture, the data, or both.",
        target: '[data-tutorial="experiments-refresh-btn"]',
      },
      {
        title: 'Rule out settings as the real cause',
        body:
          "If two architectures score differently, make sure it's really the shape and not just luckier " +
          'settings — Auto-tune (on the Data tab) finds strong settings for each one automatically, so the ' +
          'comparison reflects the architectures themselves.',
        target: '[data-tutorial="autotune-btn"]',
        focusTab: { slot: 'side', tabId: 'data' },
      },
    ],
  },
  {
    id: 'batch-inference',
    title: 'Predicting a Whole File at Once',
    blurb: 'Stop checking rows one at a time — run the model on an entire dataset and get a CSV back.',
    difficulty: 'intermediate',
    minutes: 4,
    steps: [
      {
        title: 'Beyond a five-row peek',
        body:
          '"Run on first 5 rows" is great for a quick sanity check, but it\'s not how you\'d actually use a ' +
          'trained model — for that you need every row answered, not a handful. This tab has a real way to ' +
          'do that too.',
        target: '[data-tutorial="tab-predict"]',
        focusTab: { slot: 'bottom', tabId: 'predict' },
      },
      {
        title: 'One pass, every row',
        body:
          '"Run on entire dataset & export CSV…" runs the exact same trained model, the exact same way, just ' +
          'once per row across the whole file instead of five — a real batch inference pass, not a bigger ' +
          'preview.',
        target: '[data-tutorial="batch-predict-btn"]',
      },
      {
        title: 'You choose where it lands',
        body:
          "Clicking it opens a normal save dialog — pick anywhere on your computer. Nothing is written until " +
          "you confirm a destination, and the file it writes is a plain CSV: every input column, plus a new " +
          '"prediction" column, one line per row.',
        target: '[data-tutorial="batch-predict-btn"]',
      },
      {
        title: 'Confirming it actually ran',
        body:
          "Once it finishes, the exact row count and file path are shown right here — so you know it's real " +
          "output, not a guess, before you go open the file.",
        target: '[data-tutorial="batch-predict-result"]',
      },
      {
        title: 'From here, it travels anywhere',
        body:
          "A CSV of predictions opens in any spreadsheet app, script, or dashboard — the same hand-off " +
          'point as exporting the checkpoint itself, just for answers instead of the model that produced ' +
          'them.',
        target: '[data-tutorial="export-checkpoint-btn"]',
      },
    ],
  },
  {
    id: 'model-serving',
    title: 'Letting Other Programs Ask Your Model Things',
    blurb: 'Skip the export step — start a real local server other programs can talk to directly.',
    difficulty: 'intermediate',
    minutes: 4,
    steps: [
      {
        title: 'Exporting is one way out — serving is another',
        body:
          '"Export checkpoint…" hands over a file. Serving is different: BrainBuilder itself answers real ' +
          "questions while it's running, over the network, so nothing needs to leave the app at all.",
        target: '[data-tutorial="tab-predict"]',
        focusTab: { slot: 'bottom', tabId: 'predict' },
      },
      {
        title: 'A real server, not a simulation',
        body:
          '"Start local server" opens a genuine HTTP server on this computer — the same trained checkpoint ' +
          "every other prediction here uses, just reachable by any program that can make a web request, not " +
          "only by this app's own buttons.",
        target: '[data-tutorial="serve-model-btn"]',
      },
      {
        title: 'This machine only, on purpose',
        body:
          "The server only listens on this computer (never your whole network) — an address that answers " +
          "prediction requests is a more sensitive thing to expose than a status page, so it stays local by " +
          'default rather than asking you to think about who else might reach it.',
        target: '[data-tutorial="serve-model-btn"]',
      },
      {
        title: 'A real address, and a real example',
        body:
          "Once it's running, the exact address appears here, along with a ready-to-run curl command showing " +
          "the shape of a real request — copy it into a terminal and you'll get a real answer back, not a " +
          'mock one.',
        target: '[data-tutorial="serve-model-url"]',
      },
      {
        title: 'Off when you say so',
        body:
          "The same button stops the server just as directly as it started it — nothing keeps listening after " +
          "you're done, and starting it again later is exactly as simple as the first time.",
        target: '[data-tutorial="serve-model-btn"]',
      },
    ],
  },
  {
    id: 'early-stopping',
    title: 'Knowing When to Stop Training',
    blurb: "More epochs isn't automatically better — let BrainBuilder end the run once it stops helping.",
    difficulty: 'intermediate',
    minutes: 4,
    steps: [
      {
        title: 'Every configured epoch runs — unless you say otherwise',
        body:
          "By default, training runs every epoch you set, even long after loss has stopped meaningfully " +
          "improving. That's wasted time at best — on harder problems, extra unnecessary training can even " +
          'start hurting the model. There\'s a dial for this.',
        target: '[data-tutorial="tab-data"]',
        focusTab: { slot: 'side', tabId: 'data' },
      },
      {
        title: 'The patience dial',
        body:
          '"Early-stop patience" sits right alongside epochs and learning rate. Leave it at 0 and nothing ' +
          "changes — every epoch runs, exactly like before this existed. Set it to, say, 3, and training " +
          "stops the moment loss goes 3 epochs in a row without improving.",
        target: '[data-tutorial="training-hyperparams"]',
      },
      {
        title: 'Watch it happen live',
        body:
          "Train with patience set, then switch to this tab — the same live chart you'd watch normally, no " +
          "different so far.",
        target: '[data-tutorial="tab-metrics"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
      {
        title: 'The tell',
        body:
          'If training ends before reaching your configured epoch count, a real note appears right here, ' +
          "saying exactly which epoch it stopped at. No note means it ran the full budget — patience never " +
          'triggered, or was left at 0.',
        target: '[data-tutorial="metrics-stopped-early"]',
      },
      {
        title: 'A higher patience is more cautious, not "better"',
        body:
          "A low patience stops fast but risks quitting right before a real improvement would've shown up; a " +
          "high patience gives loss more room to wobble before giving up, at the cost of possibly running " +
          "epochs that don't help. There's no universally correct number — it depends on how noisy your " +
          "training run tends to be.",
        target: '[data-tutorial="metrics-chart"]',
      },
    ],
  },
  {
    id: 'regularization',
    title: 'Keeping Your Model From Memorizing',
    blurb: "A whole-run dial that discourages any one weight from growing too large — a second way to fight overfitting.",
    difficulty: 'intermediate',
    minutes: 5,
    steps: [
      {
        title: 'Memorizing isn\'t the same as learning',
        body:
          "A model can look great on the data it trained on by memorizing quirks of that exact data, instead " +
          "of learning the real pattern — that's overfitting, and it shows up as good training loss but poor " +
          "results on anything new. There's a whole-run dial that pushes back against it.",
        target: '[data-tutorial="tab-data"]',
        focusTab: { slot: 'side', tabId: 'data' },
      },
      {
        title: 'The weight decay dial',
        body:
          '"Weight decay" sits right below early-stop patience. Leave it at 0 and nothing changes. Set it ' +
          "above 0 and every weight in the model gets nudged a little smaller on every single training step " +
          "— not deleted, just discouraged from growing huge.",
        target: '[data-tutorial="training-hyperparams"]',
      },
      {
        title: 'Not the same thing as dropout',
        body:
          'If you\'ve used a "dropout" box on the canvas, this does something related but different: dropout ' +
          "randomly ignores part of the signal during training; weight decay shrinks the weights themselves, " +
          "every step, for the whole run. Different mechanisms, same goal — and they can be combined.",
        target: '[data-tutorial="palette"]',
      },
      {
        title: 'Too much of a good thing',
        body:
          "A small weight decay is a gentle nudge. Too large a value fights the model so hard it can't learn " +
          "the real pattern either — loss stops dropping the way it should. Watch the loss chart after " +
          "changing it, the same way you would for learning rate.",
        target: '[data-tutorial="metrics-chart"]',
      },
      {
        title: 'A knob worth comparing, not guessing',
        body:
          "Train the same graph with weight decay off, then again with a value like 0.001, and check the " +
          "History tab — a real side-by-side of whether it actually helped on your data, not just a guess " +
          'either way.',
        target: '[data-tutorial="tab-experiments"]',
        focusTab: { slot: 'bottom', tabId: 'experiments' },
      },
    ],
  },
  {
    id: 'gradient-clipping',
    title: 'Taming a Wild Training Step',
    blurb: "A whole-run dial that caps how big any single update can be — insurance against one bad batch.",
    difficulty: 'intermediate',
    minutes: 4,
    steps: [
      {
        title: 'Most steps are well-behaved. Some are not',
        body:
          'Each training step nudges the model based on its gradient — how far and which direction to move. ' +
          "Occasionally a gradient comes out unusually large, and a huge, uncorrected nudge can wreck weights " +
          "that were otherwise learning fine. There's a whole-run dial that puts a ceiling on this.",
        target: '[data-tutorial="tab-data"]',
        focusTab: { slot: 'side', tabId: 'data' },
      },
      {
        title: 'The gradient clipping dial',
        body:
          '"Gradient clipping" sits right below weight decay. Leave it at 0 and nothing changes — every step ' +
          "applies exactly as computed, same as before this existed. Set it above 0 and any step whose " +
          "combined gradient size goes over that number gets scaled back down to it before it's applied — " +
          'smaller steps pass through completely untouched.',
        target: '[data-tutorial="training-hyperparams"]',
      },
      {
        title: 'Not the same dial as the other two',
        body:
          "Weight decay shrinks the weights themselves, every step, regardless of the gradient. Early-stop " +
          'patience ends the whole run early. Gradient clipping does neither — it only steps in on the rare ' +
          'step whose gradient is unusually large, tempering that one update without touching any of the ' +
          'others.',
        target: '[data-tutorial="training-hyperparams"]',
      },
      {
        title: 'Where you\'d actually notice it',
        body:
          'On a smooth, well-behaved training run, clipping rarely triggers and the loss chart looks the same ' +
          'with or without it. Its value shows up on the runs where loss spikes wildly or diverges — clipping ' +
          "won't guarantee a fix, but it removes \"one giant bad step\" as a possible cause.",
        target: '[data-tutorial="metrics-chart"]',
        focusTab: { slot: 'bottom', tabId: 'metrics' },
      },
      {
        title: 'A knob worth comparing, not guessing',
        body:
          "Train the same graph with clipping off, then again with a value like 1.0, and check the History " +
          "tab — a real side-by-side of whether it actually helped on your data, the same way you'd check " +
          'weight decay or patience.',
        target: '[data-tutorial="tab-experiments"]',
        focusTab: { slot: 'bottom', tabId: 'experiments' },
      },
    ],
  },
];
