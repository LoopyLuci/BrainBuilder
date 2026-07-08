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
];
