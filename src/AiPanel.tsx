import {
  Box,
  Button,
  Drawer,
  DrawerBody,
  DrawerCloseButton,
  DrawerContent,
  DrawerHeader,
  DrawerOverlay,
  VStack,
  HStack,
  Text,
  Textarea,
  Select,
  IconButton,
  useToast,
  Divider,
  Spinner,
  Badge,
} from "@chakra-ui/react";
import { useState, useEffect, useRef } from "react";
import { VscSend, VscTrash } from "react-icons/vsc";

type Message = {
  role: "user" | "assistant" | "system";
  content: string;
};

type Model = {
  id: string;
  name: string;
  description: string;
  context_length: number;
  pricing: {
    prompt: string;
    completion: string;
  };
};

type AiPanelProps = {
  isOpen: boolean;
  onClose: () => void;
  darkMode: boolean;
  username: string | null;
  password: string | null;
  documentContent: string;
  onApplyEdit: (content: string) => void;
};

function AiPanel({
  isOpen,
  onClose,
  darkMode,
  username,
  password,
  documentContent,
  onApplyEdit,
}: AiPanelProps) {
  const toast = useToast();
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [models, setModels] = useState<Model[]>([]);
  const [selectedModel, setSelectedModel] = useState("");
  const [includeDocument, setIncludeDocument] = useState(true);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Load available models
  useEffect(() => {
    if (isOpen) {
      loadModels();
    }
  }, [isOpen]);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  async function loadModels() {
    try {
      const response = await fetch("/api/ai/models");
      if (!response.ok) {
        throw new Error("Failed to load models");
      }
      const data = await response.json();
      setModels(data);
      if (data.length > 0 && !selectedModel) {
        setSelectedModel(data[0].id);
      }
    } catch (error) {
      toast({
        title: "Failed to load models",
        description: error instanceof Error ? error.message : "Unknown error",
        status: "error",
        duration: 4000,
        isClosable: true,
      });
    }
  }

  async function sendMessage() {
    if (!input.trim() || !username || !password) return;

    const userMessage: Message = {
      role: "user",
      content: input,
    };

    // Build messages array with optional document context
    const messagesToSend: Message[] = [...messages, userMessage];
    
    if (includeDocument && documentContent && messages.length === 0) {
      // Add document context as system message for first message only
      messagesToSend.unshift({
        role: "system",
        content: `You are a code editing assistant. The current document contains:

${documentContent}

CRITICAL OUTPUT FORMAT RULES - FOLLOW EXACTLY:
1. OUTPUT ONLY EXECUTABLE CODE/CONFIG - No explanations, no markdown, no preambles
2. DO NOT write "Here is..." or "I've created..." or any introductory text
3. DO NOT wrap output in markdown code blocks (\`\`\`) unless the document itself uses them
4. DO NOT include installation commands (brew, apt-get, npm install, etc.) in your main output
5. START your response with the actual code/content, not with any commentary
6. END your response with the actual code/content, not with usage instructions or explanations
7. If generating multiple files, use filename comments like "# filename.ext" to separate them
8. Your ENTIRE response should be copy-paste ready to replace the document

EXAMPLE - User asks for "a Python hello world":
WRONG: "Here is a Python hello world program:\n\`\`\`python\nprint('Hello')\n\`\`\`\nTo run this..."  
CORRECT: "print('Hello, World!')"

Remember: NO markdown formatting, NO explanations, JUST the code/content!`,
      });
    }

    setMessages([...messages, userMessage]);
    setInput("");
    setIsLoading(true);

    try {
      const authHeader = btoa(`${username}:${password}`);
      const response = await fetch("/api/ai/chat", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Basic ${authHeader}`,
        },
        body: JSON.stringify({
          model: selectedModel,
          messages: messagesToSend,
          max_tokens: 4096,
          temperature: 0.7,
        }),
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || "AI request failed");
      }

      const data = await response.json();
      const assistantMessage: Message = {
        role: "assistant",
        content: data.choices[0].message.content,
      };

      setMessages([...messages, userMessage, assistantMessage]);

      // Check if response contains a code block that could be the full document
      if (detectDocumentEdit(assistantMessage.content)) {
        toast({
          title: "Document edit detected",
          description: "Click 'Apply Edit' to update the document",
          status: "info",
          duration: 5000,
          isClosable: true,
        });
      }
    } catch (error) {
      toast({
        title: "AI request failed",
        description: error instanceof Error ? error.message : "Unknown error",
        status: "error",
        duration: 4000,
        isClosable: true,
      });
      // Remove the user message since request failed
      setMessages(messages);
    } finally {
      setIsLoading(false);
    }
  }

  function detectDocumentEdit(content: string): boolean {
    // Simple heuristic: check if response contains a code block
    return content.includes("```") || content.toLowerCase().includes("here is the updated");
  }

  function extractCodeFromMessage(content: string): string {
    // Extract ALL code blocks from the message
    const codeBlockRegex = /```[\w]*\n([\s\S]*?)```/g;
    const matches = [...content.matchAll(codeBlockRegex)];
    
    if (matches.length > 0) {
      // Filter out installation commands and other non-code blocks
      const codeBlocks = matches
        .map(m => m[1].trim())
        .filter(block => {
          // Skip blocks that are just installation commands
          const lines = block.split('\n');
          if (lines.length <= 3) {
            const text = block.toLowerCase();
            if (text.includes('brew install') || 
                text.includes('apt-get install') ||
                text.includes('npm install') ||
                text.includes('pip install') ||
                text.includes('choco install')) {
              return false;
            }
          }
          return true;
        });
      
      if (codeBlocks.length > 1) {
        // Multiple code blocks - join them with file separators
        return codeBlocks.map((block, idx) => {
          const lines = block.split('\n');
          // Try to detect filename from first line comment
          const firstLine = lines[0];
          if (firstLine.startsWith('#') || firstLine.startsWith('//') || firstLine.startsWith('<!--')) {
            return block;
          }
          return `# File ${idx + 1}\n${block}`;
        }).join('\n\n');
      } else if (codeBlocks.length === 1) {
        return codeBlocks[0];
      }
    }
    
    // No code blocks found - try to clean up the raw response
    let cleaned = content;
    
    // Remove common preambles
    const preambles = [
      /^(here is|here's|i've created|i've made|i've updated|i've added|i've modified|i've corrected|i've revised).*?:\s*/gim,
      /^(updated|modified|corrected|revised|new|created)\s+(code|content|document|file|version).*?:\s*/gim,
      /^sure[,!]?\s+.*?:\s*/gim,
      /^(let me|i'll|i will).*?:\s*/gim,
    ];
    
    for (const pattern of preambles) {
      cleaned = cleaned.replace(pattern, '');
    }
    
    // Remove trailing explanations (paragraphs after the main content)
    const lines = cleaned.split('\n');
    let lastCodeLine = lines.length - 1;
    
    // Find the last line that looks like code (not explanation)
    for (let i = lines.length - 1; i >= 0; i--) {
      const line = lines[i].trim();
      if (line && !line.match(/^(this|the|you|note:|explanation:|to use|usage:)/i)) {
        lastCodeLine = i;
        break;
      }
    }
    
    return lines.slice(0, lastCodeLine + 1).join('\n').trim();
  }

  function handleApplyLastEdit() {
    const lastAssistantMessage = [...messages]
      .reverse()
      .find((m) => m.role === "assistant");

    if (lastAssistantMessage) {
      const extractedContent = extractCodeFromMessage(lastAssistantMessage.content);
      onApplyEdit(extractedContent);
      toast({
        title: "Edit applied",
        description: "Document has been updated",
        status: "success",
        duration: 3000,
        isClosable: true,
      });
    }
  }

  function clearChat() {
    setMessages([]);
    toast({
      title: "Chat cleared",
      status: "info",
      duration: 2000,
      isClosable: true,
    });
  }

  const selectedModelInfo = models.find((m) => m.id === selectedModel);

  return (
    <Drawer isOpen={isOpen} placement="right" onClose={onClose} size="lg">
      <DrawerOverlay />
      <DrawerContent bgColor={darkMode ? "#1e1e1e" : "white"}>
        <DrawerCloseButton color={darkMode ? "#cbcaca" : "inherit"} />
        <DrawerHeader
          borderBottomWidth="1px"
          borderColor={darkMode ? "#3c3c3c" : "gray.200"}
          color={darkMode ? "#cbcaca" : "inherit"}
        >
          <VStack align="stretch" spacing={2}>
            <Text>Ask AI</Text>
            <Select
              size="sm"
              value={selectedModel}
              onChange={(e) => setSelectedModel(e.target.value)}
              bgColor={darkMode ? "#3c3c3c" : "white"}
              borderColor={darkMode ? "#3c3c3c" : "gray.200"}
              color={darkMode ? "#cbcaca" : "inherit"}
            >
              {models.map((model) => (
                <option key={model.id} value={model.id}>
                  {model.name}
                </option>
              ))}
            </Select>
            {selectedModelInfo && (
              <Text fontSize="xs" color={darkMode ? "#888" : "gray.600"}>
                {selectedModelInfo.description} • Context: {selectedModelInfo.context_length.toLocaleString()} tokens
              </Text>
            )}
          </VStack>
        </DrawerHeader>

        <DrawerBody p={0} display="flex" flexDirection="column">
          {/* Messages */}
          <Box flex={1} overflowY="auto" p={4}>
            {messages.length === 0 ? (
              <VStack spacing={3} py={8} color={darkMode ? "#888" : "gray.500"}>
                <Text fontSize="sm" textAlign="center">
                  Start a conversation with AI to get help with your document.
                </Text>
                <Text fontSize="xs" textAlign="center">
                  The AI can help with editing, refactoring, explaining code, and more.
                </Text>
              </VStack>
            ) : (
              <VStack spacing={4} align="stretch">
                {messages
                  .filter((m) => m.role !== "system")
                  .map((message, index) => (
                    <Box
                      key={index}
                      alignSelf={message.role === "user" ? "flex-end" : "flex-start"}
                      maxW="85%"
                    >
                      <Badge
                        colorScheme={message.role === "user" ? "blue" : "green"}
                        mb={1}
                        fontSize="xs"
                      >
                        {message.role === "user" ? "You" : "AI"}
                      </Badge>
                      <Box
                        bgColor={
                          message.role === "user"
                            ? darkMode
                              ? "#2b5278"
                              : "blue.50"
                            : darkMode
                            ? "#3c3c3c"
                            : "gray.50"
                        }
                        color={darkMode ? "#cbcaca" : "inherit"}
                        p={3}
                        borderRadius="md"
                        fontSize="sm"
                        whiteSpace="pre-wrap"
                      >
                        {message.content}
                      </Box>
                    </Box>
                  ))}
                {isLoading && (
                  <HStack spacing={2} color={darkMode ? "#888" : "gray.500"}>
                    <Spinner size="sm" />
                    <Text fontSize="sm">AI is thinking...</Text>
                  </HStack>
                )}
                <div ref={messagesEndRef} />
              </VStack>
            )}
          </Box>

          <Divider borderColor={darkMode ? "#3c3c3c" : "gray.200"} />

          {/* Action buttons */}
          {messages.some((m) => m.role === "assistant") && (
            <HStack p={2} spacing={2} borderBottomWidth="1px" borderColor={darkMode ? "#3c3c3c" : "gray.200"}>
              <Button
                size="sm"
                colorScheme="green"
                onClick={handleApplyLastEdit}
                flex={1}
              >
                Apply Last Edit
              </Button>
              <IconButton
                size="sm"
                icon={<VscTrash />}
                aria-label="Clear chat"
                onClick={clearChat}
                colorScheme="red"
                variant="ghost"
              />
            </HStack>
          )}

          {/* Input area */}
          <HStack p={4} spacing={2}>
            <Textarea
              placeholder="Ask AI anything about your document..."
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyPress={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  sendMessage();
                }
              }}
              bgColor={darkMode ? "#3c3c3c" : "white"}
              borderColor={darkMode ? "#3c3c3c" : "gray.200"}
              color={darkMode ? "#cbcaca" : "inherit"}
              rows={3}
              resize="none"
              isDisabled={isLoading}
            />
            <IconButton
              icon={isLoading ? <Spinner size="sm" /> : <VscSend />}
              aria-label="Send message"
              colorScheme="blue"
              onClick={sendMessage}
              isDisabled={!input.trim() || isLoading}
              size="lg"
            />
          </HStack>
        </DrawerBody>
      </DrawerContent>
    </Drawer>
  );
}

export default AiPanel;
