import {
  Button,
  Modal,
  ModalBody,
  ModalCloseButton,
  ModalContent,
  ModalFooter,
  ModalHeader,
  ModalOverlay,
  Input,
  VStack,
  Text,
  FormControl,
  FormLabel,
  FormErrorMessage,
  Tabs,
  TabList,
  TabPanels,
  Tab,
  TabPanel,
  useToast,
  Checkbox,
} from "@chakra-ui/react";
import { useState } from "react";

type LoginModalProps = {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: (username: string, password: string) => void;
  darkMode: boolean;
};

function LoginModal({
  isOpen,
  onClose,
  onSuccess,
  darkMode,
}: LoginModalProps) {
  const toast = useToast();
  const [isLoading, setIsLoading] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [tabIndex, setTabIndex] = useState(0);
  const [aiEnabled, setAiEnabled] = useState(false);

  // Validation
  const usernameError = username.length > 0 && username.length < 3
    ? "Username must be at least 3 characters"
    : "";
  const passwordError = password.length > 0 && password.length < 6
    ? "Password must be at least 6 characters"
    : "";
  const confirmError = tabIndex === 1 && password !== confirmPassword && confirmPassword.length > 0
    ? "Passwords do not match"
    : "";

  async function handleLogin() {
    if (usernameError || passwordError) return;

    setIsLoading(true);
    try {
      const response = await fetch("/api/auth/login", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ username, password }),
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || "Login failed");
      }

      const data = await response.json();
      localStorage.setItem("rustpad_ai_enabled", String(data.ai_enabled));

      toast({
        title: "Login successful",
        description: `Welcome back, ${username}!`,
        status: "success",
        duration: 3000,
        isClosable: true,
      });

      onSuccess(username, password);
      onClose();
      resetForm();
    } catch (error) {
      toast({
        title: "Login failed",
        description: error instanceof Error ? error.message : "Invalid credentials",
        status: "error",
        duration: 4000,
        isClosable: true,
      });
    } finally {
      setIsLoading(false);
    }
  }

  async function handleRegister() {
    if (usernameError || passwordError || confirmError || password !== confirmPassword) {
      return;
    }

    setIsLoading(true);
    try {
      const response = await fetch("/api/auth/register", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ username, password, ai_enabled: aiEnabled }),
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || "Registration failed");
      }

      const data = await response.json();
      localStorage.setItem("rustpad_ai_enabled", String(data.ai_enabled));

      toast({
        title: "Registration successful",
        description: `Welcome, ${username}! You can now freeze documents${data.ai_enabled ? " and use AI features" : ""}.`,
        status: "success",
        duration: 4000,
        isClosable: true,
      });

      onSuccess(username, password);
      onClose();
      resetForm();
    } catch (error) {
      toast({
        title: "Registration failed",
        description: error instanceof Error ? error.message : "Unable to create account",
        status: "error",
        duration: 4000,
        isClosable: true,
      });
    } finally {
      setIsLoading(false);
    }
  }

  function resetForm() {
    setUsername("");
    setPassword("");
    setConfirmPassword("");
    setTabIndex(0);
    setAiEnabled(false);
  }

  function handleClose() {
    resetForm();
    onClose();
  }

  return (
    <Modal isOpen={isOpen} onClose={handleClose} size="md">
      <ModalOverlay />
      <ModalContent bgColor={darkMode ? "#1e1e1e" : "white"}>
        <ModalHeader color={darkMode ? "#cbcaca" : "inherit"}>
          Authentication Required
        </ModalHeader>
        <ModalCloseButton color={darkMode ? "#cbcaca" : "inherit"} />
        <ModalBody>
          <Text fontSize="sm" mb={4} color={darkMode ? "#888" : "gray.600"}>
            You need to login or register to freeze documents and access your files.
          </Text>

          <Tabs index={tabIndex} onChange={setTabIndex} colorScheme="blue">
            <TabList>
              <Tab color={darkMode ? "#cbcaca" : "inherit"}>Login</Tab>
              <Tab color={darkMode ? "#cbcaca" : "inherit"}>Register</Tab>
            </TabList>

            <TabPanels>
              <TabPanel>
                <VStack spacing={4}>
                  <FormControl isInvalid={!!usernameError}>
                    <FormLabel color={darkMode ? "#cbcaca" : "inherit"}>
                      Username
                    </FormLabel>
                    <Input
                      placeholder="Enter username"
                      value={username}
                      onChange={(e) => setUsername(e.target.value)}
                      bgColor={darkMode ? "#3c3c3c" : "white"}
                      borderColor={darkMode ? "#3c3c3c" : "gray.200"}
                      color={darkMode ? "#cbcaca" : "inherit"}
                      onKeyPress={(e) => {
                        if (e.key === "Enter") handleLogin();
                      }}
                    />
                    <FormErrorMessage>{usernameError}</FormErrorMessage>
                  </FormControl>

                  <FormControl isInvalid={!!passwordError}>
                    <FormLabel color={darkMode ? "#cbcaca" : "inherit"}>
                      Password
                    </FormLabel>
                    <Input
                      type="password"
                      placeholder="Enter password"
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      bgColor={darkMode ? "#3c3c3c" : "white"}
                      borderColor={darkMode ? "#3c3c3c" : "gray.200"}
                      color={darkMode ? "#cbcaca" : "inherit"}
                      onKeyPress={(e) => {
                        if (e.key === "Enter") handleLogin();
                      }}
                    />
                    <FormErrorMessage>{passwordError}</FormErrorMessage>
                  </FormControl>

                  <Button
                    colorScheme="blue"
                    onClick={handleLogin}
                    isLoading={isLoading}
                    isDisabled={!username || !password || !!usernameError || !!passwordError}
                    w="full"
                  >
                    Login
                  </Button>
                </VStack>
              </TabPanel>

              <TabPanel>
                <VStack spacing={4}>
                  <FormControl isInvalid={!!usernameError}>
                    <FormLabel color={darkMode ? "#cbcaca" : "inherit"}>
                      Username
                    </FormLabel>
                    <Input
                      placeholder="Choose username (min 3 chars)"
                      value={username}
                      onChange={(e) => setUsername(e.target.value)}
                      bgColor={darkMode ? "#3c3c3c" : "white"}
                      borderColor={darkMode ? "#3c3c3c" : "gray.200"}
                      color={darkMode ? "#cbcaca" : "inherit"}
                    />
                    <FormErrorMessage>{usernameError}</FormErrorMessage>
                  </FormControl>

                  <FormControl isInvalid={!!passwordError}>
                    <FormLabel color={darkMode ? "#cbcaca" : "inherit"}>
                      Password
                    </FormLabel>
                    <Input
                      type="password"
                      placeholder="Choose password (min 6 chars)"
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      bgColor={darkMode ? "#3c3c3c" : "white"}
                      borderColor={darkMode ? "#3c3c3c" : "gray.200"}
                      color={darkMode ? "#cbcaca" : "inherit"}
                    />
                    <FormErrorMessage>{passwordError}</FormErrorMessage>
                  </FormControl>

                  <FormControl isInvalid={!!confirmError}>
                    <FormLabel color={darkMode ? "#cbcaca" : "inherit"}>
                      Confirm Password
                    </FormLabel>
                    <Input
                      type="password"
                      placeholder="Confirm password"
                      value={confirmPassword}
                      onChange={(e) => setConfirmPassword(e.target.value)}
                      bgColor={darkMode ? "#3c3c3c" : "white"}
                      borderColor={darkMode ? "#3c3c3c" : "gray.200"}
                      color={darkMode ? "#cbcaca" : "inherit"}
                      onKeyPress={(e) => {
                        if (e.key === "Enter") handleRegister();
                      }}
                    />
                    <FormErrorMessage>{confirmError}</FormErrorMessage>
                  </FormControl>

                  <FormControl>
                    <Checkbox
                      isChecked={aiEnabled}
                      onChange={(e) => setAiEnabled(e.target.checked)}
                      colorScheme="blue"
                      size="sm"
                    >
                      <Text fontSize="sm" color={darkMode ? "#cbcaca" : "inherit"}>
                        Enable AI features (admin only)
                      </Text>
                    </Checkbox>
                  </FormControl>

                  <Button
                    colorScheme="blue"
                    onClick={handleRegister}
                    isLoading={isLoading}
                    isDisabled={
                      !username ||
                      !password ||
                      !confirmPassword ||
                      !!usernameError ||
                      !!passwordError ||
                      !!confirmError
                    }
                    w="full"
                  >
                    Register
                  </Button>
                </VStack>
              </TabPanel>
            </TabPanels>
          </Tabs>
        </ModalBody>

        <ModalFooter>
          <Text fontSize="xs" color={darkMode ? "#888" : "gray.500"}>
            Guest users can edit documents without logging in
          </Text>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}

export default LoginModal;
