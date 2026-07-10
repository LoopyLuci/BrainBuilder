// Plain-English explanations of ML/BrainBuilder jargon, written for someone
// with zero prior knowledge (a curious kid, or an adult who's never trained
// a model). Shared by inline HelpTips and the Learn panel's manual section —
// one source of truth so a term is never explained two different ways.

export interface GlossaryEntry {
  term: string;
  short: string;
  long: string;
}

export const GLOSSARY: Record<string, GlossaryEntry> = {
  'neural-network': {
    term: 'Neural network',
    short: 'A model made of many tiny decision-makers stacked in layers.',
    long:
      "A neural network is a bunch of simple math \"neurons\" arranged in layers. Each neuron looks at the " +
      'numbers coming in, does a tiny calculation, and passes a number on to the next layer. No single neuron ' +
      'is smart on its own — but thousands of them working together, after seeing lots of examples, can learn ' +
      'to recognize pictures, sort words, or predict numbers.',
  },
  architecture: {
    term: 'Architecture',
    short: "The specific arrangement of boxes that make up a model.",
    long:
      'Architecture just means which boxes a model uses and in what order — "linear → relu → linear" is a ' +
      'different architecture than a single "linear" box alone, even trained on the exact same data. Two ' +
      "models can use identical settings and still perform differently purely because their architectures " +
      "differ — comparing across architectures, not just settings, is how you find that out.",
  },
  layer: {
    term: 'Layer',
    short: 'One step in the chain that transforms your data.',
    long:
      'Data flows through a model one layer at a time — like an assembly line. Each layer (shown as a node ' +
      'on the canvas) takes the previous layer\'s output and transforms it a little more, until the last layer ' +
      'produces the final answer (e.g. "this is a cat").',
  },
  training: {
    term: 'Training',
    short: "Showing the model examples so it learns, like practicing.",
    long:
      'Training is the model looking at your labeled examples over and over, checking how wrong its guesses ' +
      'are, and nudging its internal numbers to be a little less wrong each time. Do this thousands of times ' +
      'and the guesses get good — the same way you get better at a video game the more you play it.',
  },
  epoch: {
    term: 'Epoch',
    short: 'One full pass through all your examples.',
    long:
      'An epoch is one complete trip through every example in your dataset. Training usually takes many ' +
      'epochs — like reading a stack of flashcards over and over until you know them all by heart.',
  },
  loss: {
    term: 'Loss',
    short: 'A score for how wrong the model currently is. Lower is better.',
    long:
      "Loss is a single number that measures how far off the model's guesses are from the right answers. " +
      "It starts high (the model is guessing randomly) and should drop as training goes on — that's the " +
      'model getting better. The loss chart going down and flattening out means training is working.',
  },
  hyperparameter: {
    term: 'Hyperparameter',
    short: 'A setting you choose before training, like a recipe amount.',
    long:
      "A hyperparameter is a knob you (or BrainBuilder) set before training starts — like how many layers " +
      "to use, or how big of a step to take when learning. They're different from what the model learns on " +
      "its own; think of them as the recipe, while training is the cooking.",
  },
  'learning-rate': {
    term: 'Learning rate',
    short: 'How big a step the model takes each time it corrects itself.',
    long:
      'Too big a learning rate and the model overcorrects and never settles down. Too small and it learns ' +
      "painfully slowly. BrainBuilder picks a sensible default for you, so you don't have to guess.",
  },
  checkpoint: {
    term: 'Checkpoint',
    short: "A saved snapshot of everything your model has learned so far.",
    long:
      "A checkpoint is a save file for your trained model — like a save-game. Once you have one, you can " +
      'load it later and use the model to make predictions without retraining from scratch.',
  },
  'transfer-learning': {
    term: 'Transfer learning',
    short: 'Starting from a model that already learned something similar.',
    long:
      "Instead of learning everything from zero, transfer learning starts from a model that's already good " +
      "at a related task (like recognizing general shapes and edges in pictures), and only teaches it the " +
      "new, specific part. It learns faster and needs fewer examples — like learning a new sport when you " +
      "already know a similar one.",
  },
  classification: {
    term: 'Classification',
    short: 'Sorting things into named categories.',
    long:
      'Classification means teaching the model to sort your data into groups you name — like "cat" vs ' +
      '"dog", or "happy message" vs "sad message". The model picks one category for each new example it sees.',
  },
  node: {
    term: 'Node',
    short: 'A box on the canvas — one building block of your model.',
    long:
      'Every box on the canvas is a "node": one component of your model, like a layer or an activation. ' +
      'Nodes connect to each other with lines (edges) showing which way data flows.',
  },
  regression: {
    term: 'Regression',
    short: 'Predicting a number instead of a category.',
    long:
      'Regression means teaching the model to predict a number — like a price, a temperature, or a score — ' +
      "instead of picking a category. Instead of \"right or wrong\", the model is graded on how close its " +
      'number is to the real one.',
  },
  'text-data': {
    term: 'Text data',
    short: 'Words and sentences your model can learn to read.',
    long:
      'Text data means your examples are made of words — like product reviews, messages, or sentences in a ' +
      'spreadsheet column. BrainBuilder turns each word into numbers the model can learn from, so it can ' +
      'sort or score text the same way it sorts pictures.',
  },
  'smoke-test': {
    term: 'Smoke test',
    short: 'A quick trial run to check something basically works before trusting it.',
    long:
      "A smoke test feeds a new, computer-written piece of code a tiny fake input and checks the output has " +
      "the right shape — like plugging in a lamp just to see if it turns on, before wiring it into anything " +
      "important. BrainBuilder never adds AI-generated code to your palette unless its smoke test passes.",
  },
  worktree: {
    term: 'Worktree',
    short: 'A separate, safe copy of your project where risky changes happen first.',
    long:
      "A worktree is like a sandboxed duplicate of your project folder — changes made there don't touch your " +
      "real files until you say so. BrainBuilder's self-building agent always works in a worktree, so you can " +
      'review or throw away everything it did with zero risk to your actual work.',
  },
  cluster: {
    term: 'Cluster',
    short: 'A group of paired devices that train one model together.',
    long:
      "A Cluster is a set of devices — computers on the same network — paired together so they can train " +
      'the same model as a team. One device is the Manager, coordinating the rest. Splitting the work across ' +
      'a Cluster can train a model faster than any one device could alone.',
  },
  gradient: {
    term: 'Gradient',
    short: "The direction and size of the nudge each number gets during training.",
    long:
      'A gradient is the answer to "which way, and how much, should I adjust this number to be a little less ' +
      'wrong?" — one is calculated for every learnable number in the model, every training step. In a ' +
      'Cluster, each device calculates its own gradients and they get averaged together each round, so every ' +
      "device's nudge counts.",
  },
  embedding: {
    term: 'Embedding',
    short: 'Turning a word into a list of numbers the model can learn from.',
    long:
      "Models only understand numbers, not letters — so an embedding is a lookup table that converts each " +
      'word (or category) into a list of numbers. Similar words end up with similar numbers as the model ' +
      "trains, which is how it learns that \"great\" and \"excellent\" mean roughly the same thing.",
  },
  tokenization: {
    term: 'Tokenization',
    short: 'Splitting text into small pieces before turning them into numbers.',
    long:
      'Before text can become numbers, it first gets chopped into pieces — usually words or word-fragments, ' +
      'called tokens. "I loved it" might become three tokens: "I", "loved", "it". Each token then gets looked ' +
      'up in the embedding table.',
  },
  preprocessing: {
    term: 'Preprocessing',
    short: 'Automatic cleanup steps that run on your data before training.',
    long:
      'Preprocessing steps reshape a column of your data before the model ever sees it — for example, ' +
      '"normalize" rescales a column so its numbers average to zero, which helps training go smoothly. ' +
      'They run in order every time you train or predict, so the same cleanup always happens automatically.',
  },
  'data-augmentation': {
    term: 'Data augmentation',
    short: 'Getting more variety out of the photos you already have.',
    long:
      'Data augmentation makes small, harmless changes to some of your training examples — like mirroring a ' +
      'photo left-to-right — so the model sees more variety without you needing to collect a single extra ' +
      "photo. It teaches the model what your subject looks like in general, not just one exact orientation.",
  },
  experiment: {
    term: 'Experiment',
    short: 'One training run, with whatever settings you used for it.',
    long:
      'An "experiment" just means one attempt at training — a particular combination of settings (learning ' +
      'rate, optimizer, epochs) and the result it produced. Comparing several experiments side by side is how ' +
      'you find out which settings actually work best, instead of guessing.',
  },
  deployment: {
    term: 'Deployment',
    short: 'Taking a trained model out of BrainBuilder to actually use it.',
    long:
      'Deployment just means getting your trained model out into the world — a script, a website, another ' +
      "program — instead of leaving it sitting inside BrainBuilder. Exporting your checkpoint is the hand-off " +
      'point: from there, any plain PyTorch code can load it and put it to work.',
  },
  validation: {
    term: 'Validation',
    short: "A quick structural check that catches mistakes before you waste time training.",
    long:
      'Validation checks that your boxes are wired together correctly — every required input is connected, ' +
      'and the shapes flowing between them actually match — without running any real training. It takes a ' +
      "fraction of a second, so there's no reason not to check before committing to a full training run.",
  },
  sandbox: {
    term: 'Sandbox',
    short: 'A locked-down space where component code runs so it can never do real damage.',
    long:
      "Every component's code — even ones BrainBuilder or an AI wrote for you — runs inside a sandbox: a " +
      "restricted space that can only touch the exact files and resources it's been explicitly allowed to, " +
      "and gets killed automatically if it runs too long. The Console's \"Nervous System\" tab shows every one " +
      "of these sandboxed calls, so a denial or a crash is something you can see, not a silent mystery.",
  },
  rollback: {
    term: 'Rollback',
    short: 'Going back to a version of your model from before your last training run.',
    long:
      "A rollback undoes a training run by bringing back the checkpoint that was in place right before it — " +
      "useful when a new run made things worse instead of better. BrainBuilder keeps every checkpoint a " +
      "training run replaces, so a rollback is always available, and even a rollback can itself be undone.",
  },
  'feature-importance': {
    term: 'Feature importance',
    short: "A score for how much the model actually relies on each column of your data.",
    long:
      'Feature importance answers "which columns is the model actually using?" — BrainBuilder scrambles one ' +
      "column at a time and checks how much the model's answers change. Barely changing means that column " +
      "isn't doing much; a big change means the model leans on it heavily. It's a real test run against your " +
      "own trained model, not a guess.",
  },
  'batch-inference': {
    term: 'Batch inference',
    short: 'Running the model on a whole file of new rows at once, not just one preview.',
    long:
      'Batch inference (also called bulk prediction) means running your trained model over every row of a ' +
      "file in one go and saving all the answers — instead of checking a handful of rows by hand. It's the " +
      'same forward pass Predict already uses, just run once per row across an entire dataset and written out ' +
      'as a CSV file you can open in a spreadsheet.',
  },
  regularization: {
    term: 'Regularization',
    short: 'Discouraging any one weight from growing too large, to fight overfitting.',
    long:
      'A model can "cheat" by growing a few weights huge to memorize quirks of your specific training data ' +
      "instead of learning the general pattern — that's overfitting. Weight decay is a whole-run dial that " +
      "nudges every weight a little smaller on each step, which discourages that kind of memorizing without " +
      'changing what the model is built from. This is different from a dropout box on the canvas, which ' +
      "randomly ignores part of the signal during training — both fight overfitting, but by different means, " +
      'and can be used together.',
  },
  'early-stopping': {
    term: 'Early stopping',
    short: "Ending training automatically once more epochs stop helping.",
    long:
      "Training for more epochs isn't automatically better — past some point, loss stops meaningfully " +
      "improving, and further training just costs time (or, on harder problems, starts overfitting). Early " +
      'stopping watches the loss for you: set a "patience," and BrainBuilder stops training once that many ' +
      'epochs in a row fail to improve on the best loss seen so far, instead of always running every epoch ' +
      'you originally configured whether it\'s still helping or not.',
  },
  dropout: {
    term: 'Dropout',
    short: 'A canvas box that randomly ignores part of the signal — but only while training.',
    long:
      'A dropout box has one setting, "p" — the fraction of values it randomly zeroes out every time data ' +
      'passes through it. This forces the rest of the model to not over-rely on any single path, which fights ' +
      'overfitting (the same problem weight decay targets, by a different means — the two can be combined). ' +
      'The randomness only happens during training: once you\'re done and asking for a real prediction, ' +
      'dropout switches off automatically and passes every value through unchanged, so the same input always ' +
      "gives the same answer. This train/predict distinction — often called \"eval mode\" — isn't unique to " +
      "dropout, but dropout is the component in BrainBuilder's palette where it actually matters.",
  },
  lora: {
    term: 'LoRA (low-rank adapter)',
    short: 'A cheap way to fine-tune a big pretrained weight by training two small matrices instead of the whole thing.',
    long:
      'A "lora_linear" box on the canvas has a big frozen base weight — never updated by training — plus two ' +
      'small "adapter" matrices (their combined size is controlled by "rank") that are the only thing the ' +
      'optimizer actually touches. The adapters learn a small correction that gets added on top of the ' +
      'frozen base\'s output; "alpha" controls how strongly that correction counts. Point the base at a real ' +
      'pretrained tensor (from a local model file — see "Use Models You Already Have") and you get real ' +
      'transfer learning: almost all of the model\'s learned knowledge stays fixed, and only a tiny, cheap ' +
      'adapter has to be trained on your data.',
  },
  'learning-rate-decay': {
    term: 'Learning rate decay',
    short: 'Automatically taking smaller steps as training goes on, instead of the same size step the whole time.',
    long:
      'Early in training, the model is far from a good answer, so large steps make sense. Later, once it\'s ' +
      "roughly in the right place, the same large step can overshoot and bounce around instead of settling " +
      'in. Learning rate decay halves the learning rate every so many epochs, so later steps are ' +
      "automatically more careful than earlier ones — you set the starting learning rate once, and the " +
      "schedule handles tapering it off from there. It's a different lever from weight decay (which shrinks " +
      "the weights themselves) despite the similar name — this one only changes step size.",
  },
  'llm-authoring': {
    term: 'AI-authored graphs',
    short: 'Describe any architecture in plain English and let a language model design it — checked before it ever appears.',
    long:
      'The Build tab\'s task dropdown covers the common cases — sort into categories, predict a number — by ' +
      'picking from a small set of known shapes. The Author tab is different: type any architecture in plain ' +
      'English, and a language model (running fully locally via Ollama, or a hosted provider like OpenCode ' +
      'Go) proposes a real graph for it. Nothing it invents is trusted blindly — the result is checked against ' +
      "your real component library before it ever reaches the canvas, so a component name that doesn't exist " +
      "gets rejected instead of silently added.",
  },
  plugins: {
    term: 'Plugins',
    short: 'Extra panels you can load into the app while it\'s running, without a rebuild or reinstall.',
    long:
      'Every panel you\'ve used so far — Build, Data, Predict — was compiled into the app. A plugin is a ' +
      'small piece of code loaded live instead: click "Load" and its panel appears in the side or bottom rail ' +
      'immediately, no restart. A plugin only gets to do what it explicitly declares up front — one that only ' +
      'asks to add a panel can\'t also read your graph or call the AI, mirroring the same "ask for exactly ' +
      "what you need, nothing more\" rule the Nervous System enforces for a component's own code.",
  },
  'opencode-connect': {
    term: 'Connecting a hosted provider',
    short: 'Adding a paid, hosted AI provider\'s API key — stored in your OS keychain, never in the app itself.',
    long:
      'Ollama runs fully on your own computer, no account needed. OpenCode Go is the opposite: a hosted ' +
      "service with its own larger models, reached over the network, that needs an API key to use. Pasting " +
      'that key in and clicking "Connect" writes it straight to your operating system\'s own secure keychain ' +
      '— never to a plain file, never to browser storage, and the app never reads the raw key back afterward, ' +
      'only whether one is currently saved. "Disconnect" removes it the same way it went in.',
  },
  'gradient-clipping': {
    term: 'Gradient clipping',
    short: "Capping how big a single training step is allowed to be, so one bad batch can't blow up the model.",
    long:
      "Each training step nudges the model's weights based on its gradient — a measure of which direction " +
      "and how far to move them. Occasionally a gradient comes out unusually large (a weird batch, an early, " +
      "still-unstable step), and a huge, uncorrected nudge can wreck weights that were otherwise learning " +
      'fine — sometimes hard enough that loss never recovers. Gradient clipping sets a ceiling: if the ' +
      "combined size of a step's gradient goes over it, the whole update is scaled back down to that ceiling " +
      "before it's applied — smaller updates pass through completely untouched. It's a stability dial, " +
      'separate from weight decay (which shrinks weights, not gradients) and early stopping (which ends the ' +
      'run, rather than tempering individual steps).',
  },
  serving: {
    term: 'Serving',
    short: 'Letting other programs ask your model for predictions over the network, live.',
    long:
      'Serving means BrainBuilder itself answers real prediction requests while it runs — a script, another ' +
      'app, or a command like curl can ask it a question and get a real answer back, without you exporting a ' +
      "file first. It's the same trained model either way; serving just means the model stays inside " +
      'BrainBuilder and other programs come to it instead.',
  },
  shape: {
    term: 'Shape',
    short: 'The size/dimensions of the numbers flowing between nodes.',
    long:
      "A tensor's \"shape\" describes how many numbers it holds and how they're arranged — like an image " +
      'being 32×32 pixels with 3 color channels. Connected nodes need matching shapes, the same way plumbing ' +
      "pipes need to match up — BrainBuilder checks this for you automatically.",
  },
};

export function getGlossaryEntry(key: string): GlossaryEntry | undefined {
  return GLOSSARY[key];
}
