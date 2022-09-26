<?php

namespace Christian\Example;

use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Twig\Environment;
use Twig\Loader\FilesystemLoader;


class foo {
    public $name = "foo";
}

#[AsCommand(
    name: 'render',
    description: 'renders a template',
    hidden: false
)]
class RenderCommand extends Command
{
    protected function execute(InputInterface $input, OutputInterface $output)
    {
        $loader = new FilesystemLoader(__DIR__ . '/../Resources/views/test');
//        $output->writeln(\hello_world($name));
        $output->writeln(getcwd());
        $data = [
            'foo' => new foo(),
            'coll' => ['first', 'second', 'third'],
        ];
        $start = microtime(true);
        for ($i = 0; $i < 10000; $i++) {
//            $output->write(\read_file(__DIR__ . '/../Resources/views/test/','extend_extend_basic.html.twig', $data));
            $twig = new Environment($loader,[]);
            $template = $twig->load('extend_extend_basic.html.twig');
            $output->write($template->render($data));
        }
        $end = microtime(true);
        $output->writeln("Time: " . ($end - $start));
        return Command::SUCCESS;
    }
}