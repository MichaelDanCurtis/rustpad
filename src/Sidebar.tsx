import {
  Button,
  Container,
  Flex,
  Heading,
  Input,
  InputGroup,
  InputRightElement,
  Select,
  Stack,
  Switch,
  Text,
  useToast,
  HStack,
} from "@chakra-ui/react";
import { VscSave, VscCloudDownload, VscNewFile, VscSparkle } from "react-icons/vsc";

import ConnectionStatus from "./ConnectionStatus";
import User from "./User";
import languages from "./languages.json";
import type { UserInfo } from "./rustpad";

export type SidebarProps = {
  documentId: string;
  connection: "connected" | "disconnected" | "desynchronized";
  darkMode: boolean;
  language: string;
  currentUser: UserInfo;
  users: Record<number, UserInfo>;
  onDarkModeChange: () => void;
  onLanguageChange: (language: string) => void;
  onChangeName: (name: string) => void;
  onChangeColor: () => void;
  onFreeze: () => void;
  onDownload: () => void;
  onNewDocument: () => void;
  onAskAI?: () => void;
  aiEnabled?: boolean;
};

function Sidebar({
  documentId,
  connection,
  darkMode,
  language,
  currentUser,
  users,
  onDarkModeChange,
  onLanguageChange,
  onChangeName,
  onChangeColor,
  onFreeze,
  onDownload,
  onNewDocument,
  onAskAI,
  aiEnabled,
}: SidebarProps) {
  const toast = useToast();

  // For sharing the document by link to others.
  const documentUrl = `${window.location.origin}/#${documentId}`;

  async function handleCopy() {
    await navigator.clipboard.writeText(documentUrl);
    toast({
      title: "Copied!",
      description: "Link copied to clipboard",
      status: "success",
      duration: 2000,
      isClosable: true,
    });
  }

  return (
    <Container
      w={{ base: "3xs", md: "2xs", lg: "xs" }}
      display={{ base: "none", sm: "block" }}
      bgColor={darkMode ? "#252526" : "#f3f3f3"}
      overflowY="auto"
      maxW="full"
      lineHeight={1.4}
      py={4}
    >
      <ConnectionStatus darkMode={darkMode} connection={connection} />

      <Flex justifyContent="space-between" mt={4} mb={1.5} w="full">
        <Heading size="sm">Dark Mode</Heading>
        <Switch isChecked={darkMode} onChange={onDarkModeChange} />
      </Flex>

      <Heading mt={4} mb={1.5} size="sm">
        Language
      </Heading>
      <Select
        size="sm"
        bgColor={darkMode ? "#3c3c3c" : "white"}
        borderColor={darkMode ? "#3c3c3c" : "white"}
        value={language}
        onChange={(event) => onLanguageChange(event.target.value)}
      >
        {languages.map((lang) => (
          <option key={lang} value={lang} style={{ color: "black" }}>
            {lang}
          </option>
        ))}
      </Select>

      <Heading mt={4} mb={1.5} size="sm">
        Share Link
      </Heading>
      <InputGroup size="sm">
        <Input
          readOnly
          pr="3.5rem"
          variant="outline"
          bgColor={darkMode ? "#3c3c3c" : "white"}
          borderColor={darkMode ? "#3c3c3c" : "white"}
          value={documentUrl}
        />
        <InputRightElement width="3.5rem">
          <Button
            h="1.4rem"
            size="xs"
            onClick={handleCopy}
            _hover={{ bg: darkMode ? "#575759" : "gray.200" }}
            bgColor={darkMode ? "#575759" : "gray.200"}
            color={darkMode ? "white" : "inherit"}
          >
            Copy
          </Button>
        </InputRightElement>
      </InputGroup>

      <Heading mt={4} mb={1.5} size="sm">
        Document Actions
      </Heading>
      <Button
        size="sm"
        colorScheme={darkMode ? "whiteAlpha" : "blackAlpha"}
        borderColor={darkMode ? "purple.400" : "purple.600"}
        color={darkMode ? "purple.400" : "purple.600"}
        variant="outline"
        leftIcon={<VscNewFile />}
        w="full"
        mb={2}
        onClick={onNewDocument}
      >
        Create New Document
      </Button>
      {aiEnabled && onAskAI && (
        <Button
          size="sm"
          colorScheme={darkMode ? "whiteAlpha" : "blackAlpha"}
          borderColor={darkMode ? "yellow.400" : "yellow.600"}
          color={darkMode ? "yellow.400" : "yellow.600"}
          variant="outline"
          leftIcon={<VscSparkle />}
          w="full"
          mb={2}
          onClick={onAskAI}
        >
          Ask AI
        </Button>
      )}
      <HStack spacing={2}>
        <Button
          size="sm"
          colorScheme={darkMode ? "whiteAlpha" : "blackAlpha"}
          borderColor={darkMode ? "green.400" : "green.600"}
          color={darkMode ? "green.400" : "green.600"}
          variant="outline"
          leftIcon={<VscSave />}
          flex={1}
          onClick={onFreeze}
        >
          Freeze 30d
        </Button>
        <Button
          size="sm"
          colorScheme={darkMode ? "whiteAlpha" : "blackAlpha"}
          borderColor={darkMode ? "blue.400" : "blue.600"}
          color={darkMode ? "blue.400" : "blue.600"}
          variant="outline"
          leftIcon={<VscCloudDownload />}
          flex={1}
          onClick={onDownload}
        >
          Download
        </Button>
      </HStack>

      <Heading mt={4} mb={1.5} size="sm">
        Active Users
      </Heading>
      <Stack spacing={0} mb={1.5} fontSize="sm">
        <User
          info={currentUser}
          isMe
          onChangeName={onChangeName}
          onChangeColor={onChangeColor}
          darkMode={darkMode}
        />
        {Object.entries(users).map(([id, info]) => (
          <User key={id} info={info} darkMode={darkMode} />
        ))}
      </Stack>
    </Container>
  );
}

export default Sidebar;
