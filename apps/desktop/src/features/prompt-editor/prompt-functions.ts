export type PromptArgumentCompletion = "chunk" | null;

export type PromptFunctionParameter = {
  name: string;
  required: boolean;
  acceptsNamed: boolean;
  completion: PromptArgumentCompletion;
};

export type PromptFunctionDefinition = {
  name: string;
  detailMessage: "reusableChunk" | "compileTimeComment";
  parameters: readonly PromptFunctionParameter[];
};

export const NAI_PROMPT_FUNCTIONS: readonly PromptFunctionDefinition[] = [
  {
    name: "chunk",
    detailMessage: "reusableChunk",
    parameters: [
      {
        name: "key",
        required: true,
        acceptsNamed: false,
        completion: "chunk",
      },
    ],
  },
  {
    name: "comment",
    detailMessage: "compileTimeComment",
    parameters: [
      {
        name: "text",
        required: true,
        acceptsNamed: false,
        completion: null,
      },
    ],
  },
];

export function promptFunctionDefinition(
  name: string,
  functions: readonly PromptFunctionDefinition[] = NAI_PROMPT_FUNCTIONS,
): PromptFunctionDefinition | undefined {
  return functions.find((definition) => definition.name === name);
}

export function promptFunctionAcceptsArgumentCount(
  definition: PromptFunctionDefinition,
  count: number,
): boolean {
  const minimum = definition.parameters.filter((parameter) => parameter.required).length;
  return count >= minimum && count <= definition.parameters.length;
}

export function promptFunctionParameter(
  definition: PromptFunctionDefinition,
  index: number,
  named: string | null,
): PromptFunctionParameter | undefined {
  if (named !== null) {
    return definition.parameters.find(
      (parameter) => parameter.acceptsNamed && parameter.name === named,
    );
  }
  return definition.parameters[index];
}

export function functionStartsArgumentCompletion(name: string): boolean {
  return NAI_PROMPT_FUNCTIONS.some(
    (definition) => definition.name === name && definition.parameters[0]?.completion !== null,
  );
}
