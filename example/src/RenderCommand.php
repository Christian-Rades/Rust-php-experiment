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
        $twig = new Environment($loader,[]);
        $template = $twig->load('basic.html.twig');
        $name = 'christian';
//        $output->writeln(\hello_world($name));
        $output->writeln(getcwd());
        $output->write(\read_file(__DIR__ . '/../Resources/views/test/basic.html.twig', [
            'foo' => new foo(),
            'coll' => ['first', 'second', 'third'],
        ]));
//        $output->writeln($template->render(['name' => $name]));
        return Command::SUCCESS;
    }
}