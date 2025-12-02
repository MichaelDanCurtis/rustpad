import { Flex, Icon, Text, Button } from "@chakra-ui/react";
import { VscRemote, VscFolder } from "react-icons/vsc";

const version =
  typeof import.meta.env.VITE_SHA === "string"
    ? import.meta.env.VITE_SHA.slice(0, 7)
    : "development";

type FooterProps = {
  onOpenFiles?: () => void;
};

function Footer({ onOpenFiles }: FooterProps) {
  return (
    <Flex h="22px" bgColor="#0071c3" color="white">
      <Flex
        h="100%"
        bgColor="#09835c"
        pl={2.5}
        pr={4}
        fontSize="sm"
        align="center"
      >
        <Icon as={VscRemote} mb={-0.5} mr={1} />
        <Text fontSize="xs">Rustpad ({version})</Text>
      </Flex>
      {onOpenFiles && (
        <Button
          size="xs"
          variant="ghost"
          color="white"
          leftIcon={<VscFolder />}
          onClick={onOpenFiles}
          h="100%"
          borderRadius={0}
          fontSize="xs"
          _hover={{ bgColor: "rgba(255, 255, 255, 0.1)" }}
        >
          My Files
        </Button>
      )}
    </Flex>
  );
}

export default Footer;
