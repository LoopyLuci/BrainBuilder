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
